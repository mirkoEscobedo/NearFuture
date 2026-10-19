import com.fs.starfarer.api.EveryFrameScript;
import com.fs.starfarer.api.Global;
import com.fs.starfarer.api.campaign.SectorAPI;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import nf.adapter.NearFutureModPlugin;

/** Licensed API lifecycle host only; no campaign scheduler or save serializer is simulated. */
public final class CampaignObserverAfterSaveCapture {
    private static final String ENABLE = "nf.profiling.callback.enabled";
    private static final String TRACE = "nf.profiling.traceId";
    private CampaignObserverAfterSaveCapture() { }
    private static void restore(String name, String value) {
        if (value == null) System.clearProperty(name);
        else System.setProperty(name, value);
    }
    public static void main(String[] args) {
        if (args.length != 0) throw new IllegalArgumentException();
        SectorAPI previousSector = Global.getSector();
        String previousEnable = System.getProperty(ENABLE), previousTrace = System.getProperty(TRACE);
        EveryFrameScript sentinel = new EveryFrameScript() {
            @Override public boolean isDone() { return false; }
            @Override public boolean runWhilePaused() { return false; }
            @Override public void advance(float amount) { }
        };
        List<EveryFrameScript> scripts = new ArrayList<>();
        scripts.add(sentinel);
        SectorAPI sector = (SectorAPI) Proxy.newProxyInstance(SectorAPI.class.getClassLoader(),
            new Class<?>[] {SectorAPI.class}, (proxy, method, arguments) -> {
                return switch (method.getName()) {
                    case "addTransientScript" -> { scripts.add((EveryFrameScript) arguments[0]); yield null; }
                    case "removeTransientScript" -> {
                        EveryFrameScript target = (EveryFrameScript) arguments[0];
                        scripts.removeIf(script -> script == target);
                        yield null;
                    }
                    case "getTransientScripts" -> List.copyOf(scripts);
                    case "equals" -> proxy == arguments[0];
                    case "hashCode" -> System.identityHashCode(proxy);
                    case "toString" -> "LicensedAfterSaveHostProxy";
                    default -> throw new UnsupportedOperationException("Unexpected host API: " + method.getName());
                };
            });
        NearFutureModPlugin plugin = new NearFutureModPlugin();
        int loadedCount, beforeSaveCount, resumedCount;
        boolean oldDone, sentinelBefore, replacementDistinct, sentinelAfter;
        try {
            Global.setSector(sector);
            System.setProperty(ENABLE, "true");
            System.setProperty(TRACE, "0123456789abcdef0123456789abcdef");
            plugin.onGameLoad(false);
            loadedCount = scripts.size() - 1;
            EveryFrameScript original = scripts.stream().filter(script -> script != sentinel).findFirst().orElse(null);
            plugin.beforeGameSave();
            beforeSaveCount = scripts.size() - 1;
            oldDone = original != null && original.isDone();
            sentinelBefore = scripts.size() == 1 && scripts.get(0) == sentinel && !sentinel.isDone();
            plugin.afterGameSave();
            resumedCount = scripts.size() - 1;
            EveryFrameScript replacement = scripts.stream().filter(script -> script != sentinel).findFirst().orElse(null);
            replacementDistinct = replacement != null && replacement != original && !replacement.isDone()
                && original != null && original.isDone();
            sentinelAfter = scripts.stream().anyMatch(script -> script == sentinel) && !sentinel.isDone();
        } finally {
            try { plugin.beforeGameSave(); }
            finally {
                Global.setSector(previousSector);
                restore(ENABLE, previousEnable);
                restore(TRACE, previousTrace);
            }
        }
        int finalOwnedCount = scripts.size() - 1;
        boolean sentinelFinal = scripts.size() == 1 && scripts.get(0) == sentinel && !sentinel.isDone();
        boolean restoredSector = Global.getSector() == previousSector;
        boolean restoredProperties = Objects.equals(System.getProperty(ENABLE), previousEnable)
            && Objects.equals(System.getProperty(TRACE), previousTrace);
        System.out.println("{\"loadedCount\":" + loadedCount + ",\"beforeSaveCount\":" + beforeSaveCount
            + ",\"oldDone\":" + oldDone + ",\"sentinelBefore\":" + sentinelBefore
            + ",\"resumedCount\":" + resumedCount + ",\"replacementDistinct\":" + replacementDistinct
            + ",\"sentinelAfter\":" + sentinelAfter + ",\"finalOwnedCount\":" + finalOwnedCount
            + ",\"sentinelFinal\":" + sentinelFinal + ",\"restoredSector\":" + restoredSector
            + ",\"restoredProperties\":" + restoredProperties + "}");
    }
}
