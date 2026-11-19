import com.fs.starfarer.api.EveryFrameScript;
import com.fs.starfarer.api.Global;
import com.fs.starfarer.api.campaign.SectorAPI;
import java.lang.reflect.Proxy;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordedEvent;
import jdk.jfr.consumer.RecordingFile;
import nf.adapter.NearFutureModPlugin;
import nf.adapter.profiling.CampaignCallbackObserver;

/** Real licensed API types; the host does not schedule callbacks or serialize a campaign/save. */
public final class CampaignObserverLifecycleControls {
    private static final String ENABLE = "nf.profiling.callback.enabled";
    private static final String TRACE = "nf.profiling.traceId";
    private static final String VALID = "0123456789abcdef0123456789abcdef";
    private CampaignObserverLifecycleControls() { }

    private static void property(String name, String value) {
        if (value == null) System.clearProperty(name);
        else System.setProperty(name, value);
    }
    private static final class Environment implements AutoCloseable {
        private final SectorAPI previousSector = Global.getSector();
        private final String previousEnable = System.getProperty(ENABLE);
        private final String previousTrace = System.getProperty(TRACE);
        void configure(String enabled, String trace) { property(ENABLE, enabled); property(TRACE, trace); }
        @Override public void close() {
            try { Global.setSector(previousSector); }
            finally { property(ENABLE, previousEnable); property(TRACE, previousTrace); }
        }
        void report(Map<String, Object> result) {
            result.put("restoredSector", Global.getSector() == previousSector);
            result.put("restoredProperties", Objects.equals(System.getProperty(ENABLE), previousEnable)
                && Objects.equals(System.getProperty(TRACE), previousTrace));
            result.put("previousEnableUnset", previousEnable == null);
            result.put("previousTraceUnset", previousTrace == null);
        }
    }
    private static final class Sentinel implements EveryFrameScript {
        private int advances;
        @Override public boolean isDone() { return false; }
        @Override public boolean runWhilePaused() { return false; }
        @Override public void advance(float amount) { advances++; }
    }
    private static final class Host {
        private final Sentinel sentinel = new Sentinel();
        private final List<EveryFrameScript> scripts = new ArrayList<>();
        private final List<EveryFrameScript> removed = new ArrayList<>();
        private final SectorAPI sector;
        Host() {
            scripts.add(sentinel);
            sector = (SectorAPI) Proxy.newProxyInstance(SectorAPI.class.getClassLoader(),
                new Class<?>[] {SectorAPI.class}, (proxy, method, arguments) -> switch (method.getName()) {
                    case "addTransientScript" -> { scripts.add((EveryFrameScript) arguments[0]); yield null; }
                    case "removeTransientScript" -> {
                        EveryFrameScript target = (EveryFrameScript) arguments[0];
                        removed.add(target); scripts.removeIf(script -> script == target); yield null;
                    }
                    case "getTransientScripts" -> List.copyOf(scripts);
                    case "hasTransientScript" -> scripts.stream().anyMatch(((Class<?>) arguments[0])::isInstance);
                    case "isPaused" -> false;
                    case "equals" -> proxy == arguments[0];
                    case "hashCode" -> System.identityHashCode(proxy);
                    case "toString" -> "LicensedLifecycleControlHost";
                    default -> throw new UnsupportedOperationException("Unexpected host API: " + method.getName());
                });
        }
        int ownedCount() { return (int) scripts.stream().filter(CampaignCallbackObserver.class::isInstance).count(); }
        CampaignCallbackObserver current() {
            if (ownedCount() != 1) throw new AssertionError("expected exactly one current owned observer");
            return scripts.stream().filter(CampaignCallbackObserver.class::isInstance)
                .map(CampaignCallbackObserver.class::cast).findFirst().orElseThrow();
        }
        boolean intact() {
            return scripts.stream().filter(script -> script == sentinel).count() == 1
                && !sentinel.isDone() && sentinel.advances == 0 && !removed.contains(sentinel);
        }
        boolean removedExactlyOnce(EveryFrameScript target) {
            return removed.stream().filter(script -> script == target).count() == 1 && !scripts.contains(target);
        }
    }
    private static Map<String, Object> optIn() {
        Environment environment = new Environment();
        Host host = new Host();
        NearFutureModPlugin plugin = new NearFutureModPlugin();
        Map<String, Object> result = new LinkedHashMap<>();
        List<Integer> counts = new ArrayList<>();
        boolean intact = true;
        try {
            Global.setSector(host.sector);
            String[][] cases = {{null, VALID}, {"false", VALID}, {"TRUE", VALID}, {"true ", VALID}, {"1", VALID},
                {"true", null}, {"true", "0123456789ABCDEF0123456789ABCDEF"}, {"true", VALID.substring(1)},
                {"true", VALID + "0"}, {"true", "g" + VALID.substring(1)}};
            for (String[] values : cases) {
                environment.configure(values[0], values[1]);
                plugin.onGameLoad(false);
                counts.add(host.ownedCount()); intact &= host.intact();
            }
            environment.configure("true", VALID); Global.setSector(null);
            plugin.onGameLoad(false);
            counts.add(host.ownedCount()); intact &= host.intact();
            result.put("ownedCounts", counts); result.put("sentinelIntactEveryCut", intact);
        } finally {
            try { plugin.beforeGameSave(); }
            finally { environment.close(); }
        }
        result.put("finalOwnedCount", host.ownedCount()); result.put("sentinelFinal", host.intact());
        environment.report(result); return result;
    }
    private static Map<String, Object> repeatedLoad() {
        Environment environment = new Environment();
        Host oldHost = new Host(), newHost = new Host();
        NearFutureModPlugin plugin = new NearFutureModPlugin();
        Map<String, Object> result = new LinkedHashMap<>();
        try {
            environment.configure("true", VALID); Global.setSector(oldHost.sector);
            plugin.onGameLoad(false);
            CampaignCallbackObserver first = oldHost.current();
            result.put("firstOwnedCount", oldHost.ownedCount());
            plugin.onGameLoad(true);
            CampaignCallbackObserver second = oldHost.current();
            result.put("sameSectorOwnedCount", oldHost.ownedCount());
            result.put("firstClosedRemoved", first.isDone() && oldHost.removedExactlyOnce(first));
            result.put("sameSectorReplacementDistinct", second != first && !second.isDone());
            result.put("oldSentinelAfterRepeat", oldHost.intact());
            Global.setSector(newHost.sector);
            plugin.onGameLoad(false);
            CampaignCallbackObserver third = newHost.current();
            result.put("oldSectorOwnedCount", oldHost.ownedCount());
            result.put("newSectorOwnedCount", newHost.ownedCount());
            result.put("secondClosedRemovedFromOldSector", second.isDone() && oldHost.removedExactlyOnce(second));
            result.put("currentDistinct", third != second && third != first && !third.isDone());
            result.put("bothSentinelsIntact", oldHost.intact() && newHost.intact());
        } finally {
            try { plugin.beforeGameSave(); }
            finally { environment.close(); }
        }
        result.put("finalOldCount", oldHost.ownedCount()); result.put("finalNewCount", newHost.ownedCount());
        result.put("bothSentinelsFinal", oldHost.intact() && newHost.intact());
        environment.report(result); return result;
    }
    private static Map<String, Object> afterSave(Path recordingPath) throws Exception {
        Environment environment = new Environment();
        Host host = new Host();
        NearFutureModPlugin plugin = new NearFutureModPlugin();
        Map<String, Object> result = new LinkedHashMap<>();
        List<Integer> counts = new ArrayList<>();
        try (Recording recording = new Recording()) {
            recording.setMaxSize(8 * 1024 * 1024);
            recording.setDuration(Duration.ofSeconds(5));
            recording.enable("nf.CampaignCallback").withThreshold(Duration.ZERO).withoutStackTrace();
            recording.start();
            Global.setSector(host.sector); environment.configure("false", VALID);
            plugin.onGameLoad(false); plugin.afterGameSave(); counts.add(host.ownedCount());
            environment.configure("true", VALID); plugin.afterGameSave(); counts.add(host.ownedCount());
            CampaignCallbackObserver first = host.current();
            first.advance(0f); // The one current observer emits exactly one actual callback event.
            plugin.afterGameSave(); counts.add(host.ownedCount());
            CampaignCallbackObserver second = host.current();
            result.put("firstClosedRemoved", first.isDone() && host.removedExactlyOnce(first));
            result.put("replacementDistinct", second != first && !second.isDone());
            first.advance(0f); // A stale closed script must not emit another callback.
            environment.configure("true", "invalid-trace"); plugin.afterGameSave(); counts.add(host.ownedCount());
            result.put("secondClosedRemoved", second.isDone() && host.removedExactlyOnce(second));
            second.advance(0f); first.advance(0f);
            result.put("ownedCounts", counts); result.put("sentinelIntactEveryCut", host.intact());
            recording.stop(); recording.dump(recordingPath);
        } finally {
            try { plugin.beforeGameSave(); }
            finally { environment.close(); }
        }
        long callbacks = 0;
        boolean traceMatches = true;
        for (RecordedEvent event : RecordingFile.readAllEvents(recordingPath)) {
            if (event.getEventType().getName().equals("nf.CampaignCallback")) {
                callbacks++; traceMatches &= VALID.equals(event.getString("traceId"));
            }
        }
        result.put("actualCallbackEvents", callbacks); result.put("callbackTraceMatches", traceMatches);
        result.put("recordingBytes", Files.size(recordingPath));
        result.put("finalOwnedCount", host.ownedCount()); result.put("sentinelFinal", host.intact());
        environment.report(result); return result;
    }
    private static String json(Object value) {
        if (value instanceof Boolean || value instanceof Number) return value.toString();
        if (value instanceof List<?> values) return "[" + String.join(",", values.stream()
            .map(CampaignObserverLifecycleControls::json).toList()) + "]";
        if (value instanceof Map<?, ?> values) {
            List<String> fields = new ArrayList<>();
            for (Map.Entry<?, ?> entry : values.entrySet()) fields.add("\"" + entry.getKey() + "\":" + json(entry.getValue()));
            return "{" + String.join(",", fields) + "}";
        }
        throw new IllegalArgumentException("unsupported closed control projection");
    }
    public static void main(String[] args) throws Exception {
        if (args.length != 2) throw new IllegalArgumentException();
        Map<String, Object> result = switch (args[0]) {
            case "optIn" -> optIn();
            case "repeatedLoad" -> repeatedLoad();
            case "afterSave" -> afterSave(Path.of(args[1]));
            default -> throw new IllegalArgumentException("unknown closed control selector");
        };
        System.out.println(json(result));
    }
}
