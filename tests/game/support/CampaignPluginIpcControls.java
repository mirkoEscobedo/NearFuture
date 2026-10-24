import com.fs.starfarer.api.EveryFrameScript;
import com.fs.starfarer.api.Global;
import com.fs.starfarer.api.campaign.SectorAPI;
import java.lang.reflect.Proxy;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicReference;
import nf.adapter.NearFutureModPlugin;
import nf.adapter.lifecycle.CampaignLifecycleScript;

/** Actual licensed callback signatures; no native frame/saving claims. */
public final class CampaignPluginIpcControls {
    private CampaignPluginIpcControls() { }
    private static void check(boolean value, String message) { if (!value) throw new AssertionError(message); }
    public static void main(String[] args) throws Exception {
        SectorAPI previous = Global.getSector();
        Map<String,String> changed = new HashMap<>();
        Map<String,String> configured = Map.ofEntries(
            Map.entry("enabled","true"), Map.entry("executable",Path.of(args[0],"missing-node").toString()),
            Map.entry("sha256","00".repeat(32)), Map.entry("vault",Path.of(args[0],"vault").toString()),
            Map.entry("saves",Path.of(args[0],"saves").toString()), Map.entry("publication","game-host-control"),
            Map.entry("universe","01".repeat(16)), Map.entry("history","02".repeat(16)),
            Map.entry("ruleset","03".repeat(32)), Map.entry("content","04".repeat(32)),
            Map.entry("account","05".repeat(16)), Map.entry("device","06".repeat(16)));
        List<EveryFrameScript> scripts = new ArrayList<>();
        EveryFrameScript sentinel = new EveryFrameScript() {
            public boolean isDone() { return false; } public boolean runWhilePaused() { return false; }
            public void advance(float amount) { throw new AssertionError("unrelated callback executed"); }
        };
        scripts.add(sentinel);
        Thread expectedThread=Thread.currentThread();
        SectorAPI sector = (SectorAPI) Proxy.newProxyInstance(SectorAPI.class.getClassLoader(),new Class<?>[]{SectorAPI.class},
            (proxy,method,arguments) -> {
                if(Thread.currentThread()!=expectedThread) throw new AssertionError("wrong-thread hook accessed game API");
                return switch(method.getName()) {
                case "addTransientScript" -> { scripts.add((EveryFrameScript)arguments[0]); yield null; }
                case "removeTransientScript" -> { scripts.removeIf(script -> script == arguments[0]); yield null; }
                default -> throw new AssertionError("unexpected game access: " + method.getName());
                };
            });
        NearFutureModPlugin plugin = new NearFutureModPlugin();
        try {
            for (var entry:configured.entrySet()) {
                String key="nf.ipc.diagnostic."+entry.getKey(); changed.put(key,System.getProperty(key));
                System.setProperty(key,entry.getValue());
            }
            Global.setSector(sector);
            plugin.onApplicationLoad();
            plugin.onGameLoad(false);
            var first = current(scripts);
            check(!first.isDone() && scripts.contains(sentinel),"missing node blocked value callback registration");
            first.advance(0);
            plugin.beforeGameSave();
            check(first.isDone() && !scripts.contains(first) && scripts.size()==1,"save must revoke and detach exact binding");
            first.advance(0);
            plugin.afterGameSave();
            var second = current(scripts);
            check(second!=first,"successful save must use a fresh binding");
            plugin.onGameLoad(false);
            var third=current(scripts);
            check(second.isDone() && third!=second,"reload must revoke previous generation");
            plugin.beforeGameSave(); plugin.onGameSaveFailed();
            check(current(scripts)!=third && third.isDone(),"failed save must rebind without reviving old values");
            var last=current(scripts);
            AtomicReference<Throwable> foreignFailure=new AtomicReference<>();
            Thread foreign=new Thread(()->{try{plugin.beforeGameSave();}catch(Throwable failure){foreignFailure.set(failure);}});
            foreign.start();foreign.join(2000);
            check(!foreign.isAlive() && foreignFailure.get()==null,"wrong-thread hook performed game effects");
            check(last.isDone(),"wrong-thread hook did not revoke diagnostics");
            plugin.beforeGameSave();
            check(scripts.size()==1 && scripts.get(0)==sentinel,"unrelated script changed");
        } finally {
            plugin.beforeGameSave(); Global.setSector(previous);
            for(var entry:changed.entrySet()) {
                if(entry.getValue()==null) System.clearProperty(entry.getKey()); else System.setProperty(entry.getKey(),entry.getValue());
            }
        }
        System.out.println("PLUGIN_MISSING_NODE_FENCES_OK");
    }
    private static CampaignLifecycleScript current(List<EveryFrameScript> scripts) {
        var owned=scripts.stream().filter(CampaignLifecycleScript.class::isInstance).toList();
        check(owned.size()==1,"one current plugin IPC script required");
        return (CampaignLifecycleScript)owned.get(0);
    }
}
