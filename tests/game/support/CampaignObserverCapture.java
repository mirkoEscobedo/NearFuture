import com.fs.starfarer.api.EveryFrameScript;
import com.fs.starfarer.api.Global;
import com.fs.starfarer.api.campaign.SectorAPI;
import java.lang.reflect.Proxy;
import java.nio.file.Path;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import jdk.jfr.Recording;
import nf.adapter.NearFutureModPlugin;

/** Licensed API host seam only; the proxy does not schedule or serialize campaign callbacks. */
public final class CampaignObserverCapture {
    private static final String ENABLE = "nf.profiling.callback.enabled";
    private static final String TRACE = "nf.profiling.traceId";
    private CampaignObserverCapture() { }

    private static void restore(String name, String value) {
        if (value == null) System.clearProperty(name);
        else System.setProperty(name, value);
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 2 || !args[1].matches("[a-f0-9]{32}")) throw new IllegalArgumentException();
        SectorAPI previousSector = Global.getSector();
        String previousEnable = System.getProperty(ENABLE);
        String previousTrace = System.getProperty(TRACE);
        List<EveryFrameScript> scripts = new ArrayList<>();
        List<EveryFrameScript> removed = new ArrayList<>();
        SectorAPI sector = (SectorAPI) Proxy.newProxyInstance(SectorAPI.class.getClassLoader(),
            new Class<?>[] {SectorAPI.class}, (proxy, method, arguments) -> {
                return switch (method.getName()) {
                    case "addTransientScript" -> { scripts.add((EveryFrameScript) arguments[0]); yield null; }
                    case "removeTransientScript" -> {
                        EveryFrameScript target = (EveryFrameScript) arguments[0];
                        removed.add(target);
                        scripts.removeIf(script -> script == target);
                        yield null;
                    }
                    case "getTransientScripts" -> List.copyOf(scripts);
                    case "hasTransientScript" -> scripts.stream().anyMatch(((Class<?>) arguments[0])::isInstance);
                    case "isPaused" -> false;
                    case "equals" -> proxy == arguments[0];
                    case "hashCode" -> System.identityHashCode(proxy);
                    case "toString" -> "LicensedSectorHostProxy";
                    default -> throw new UnsupportedOperationException("Unexpected host API: " + method.getName());
                };
            });
        int observerCount;
        int remainingCount;
        boolean removedOwned;
        long threadId = Thread.currentThread().getId();
        try (Recording recording = new Recording()) {
            recording.setMaxSize(8 * 1024 * 1024);
            recording.setDuration(Duration.ofSeconds(5));
            recording.enable("nf.CampaignCallback").withThreshold(Duration.ZERO).withoutStackTrace();
            recording.start();
            Global.setSector(sector);
            System.setProperty(ENABLE, "true");
            System.setProperty(TRACE, args[1]);
            NearFutureModPlugin plugin = new NearFutureModPlugin();
            plugin.onGameLoad(false);
            observerCount = sector.getTransientScripts().size();
            EveryFrameScript owned = observerCount == 1 ? scripts.get(0) : null;
            // No substitute event: only a retained real adapter script can produce the three spans.
            if (owned != null) for (int index = 0; index < 3; index++) owned.advance(0f);
            plugin.beforeGameSave();
            remainingCount = sector.getTransientScripts().size();
            removedOwned = owned != null && removed.size() == 1 && removed.get(0) == owned;
            recording.stop();
            recording.dump(Path.of(args[0]));
        } finally {
            Global.setSector(previousSector);
            restore(ENABLE, previousEnable);
            restore(TRACE, previousTrace);
        }
        boolean restoredSector = Global.getSector() == previousSector;
        boolean restoredProperties = Objects.equals(System.getProperty(ENABLE), previousEnable)
            && Objects.equals(System.getProperty(TRACE), previousTrace);
        System.out.println("{\"observerCount\":" + observerCount + ",\"remainingCount\":" + remainingCount
            + ",\"removedOwned\":" + removedOwned + ",\"threadId\":" + threadId
            + ",\"restoredSector\":" + restoredSector + ",\"restoredProperties\":" + restoredProperties + "}");
    }
}
