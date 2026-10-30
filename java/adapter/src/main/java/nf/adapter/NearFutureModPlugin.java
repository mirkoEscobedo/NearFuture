package nf.adapter;

import com.fs.starfarer.api.BaseModPlugin;
import com.fs.starfarer.api.Global;
import com.fs.starfarer.api.campaign.SectorAPI;
import nf.adapter.profiling.CampaignCallbackObserver;
import nf.contract.BuildIdentity;
import nf.adapter.ipc.DiagnosticConfiguration;
import nf.adapter.ipc.LifecycleOwner;
import nf.adapter.ipc.NodeFailure;
import nf.adapter.lifecycle.CampaignLifecycleScript;
import java.util.concurrent.atomic.AtomicReference;

/** Optional observer seam; no authority, recording ownership, or economic game mutation. */
public final class NearFutureModPlugin extends BaseModPlugin {
    private transient SectorAPI callbackSector;
    private transient CampaignCallbackObserver callbackObserver;
    private transient volatile LifecycleOwner ipcOwner;
    private transient SectorAPI ipcSector;
    private transient volatile CampaignLifecycleScript ipcScript;
    private transient long ipcGeneration;
    private final transient AtomicReference<Thread> callbackThread = new AtomicReference<>();

    @Override
    public void onApplicationLoad() {
        if (ipcOwner != null) return;
        try {
            ipcOwner = DiagnosticConfiguration.configuredOwner();
        } catch (NodeFailure unavailable) { /* Explicit invalid diagnostics remain disabled. */ }
    }

    public String syntheticBuildIdentity() {
        return BuildIdentity.description();
    }

    @Override
    public void onGameLoad(boolean newGame) {
        if (!callbackThreadAllowed()) return;
        detachIpc();
        attachIpc();
        detachObserver();
        if (!"true".equals(System.getProperty("nf.profiling.callback.enabled"))) return;
        String traceId = System.getProperty("nf.profiling.traceId");
        if (traceId == null || !traceId.matches("[a-f0-9]{32}")) return;
        SectorAPI sector = Global.getSector();
        if (sector == null) return;
        CampaignCallbackObserver observer = new CampaignCallbackObserver(traceId);
        sector.addTransientScript(observer);
        callbackSector = sector;
        callbackObserver = observer;
    }

    @Override
    public void afterGameSave() {
        onGameLoad(false);
    }

    @Override
    public void onGameSaveFailed() {
        onGameLoad(false);
    }

    @Override
    public void beforeGameSave() {
        if (!callbackThreadAllowed()) return;
        detachIpc();
        detachObserver();
    }

    private boolean callbackThreadAllowed() {
        Thread caller = Thread.currentThread();
        callbackThread.compareAndSet(null, caller);
        if (callbackThread.get() == caller) return true;
        // A foreign invocation may revoke values, but must not access SectorAPI.
        if (ipcScript != null) ipcScript.close();
        if (ipcOwner != null) ipcOwner.shutdown();
        return false;
    }

    private void attachIpc() {
        if (ipcOwner == null) return;
        if (ipcGeneration == Long.MAX_VALUE) { ipcOwner.shutdown(); return; }
        SectorAPI sector = Global.getSector();
        if (sector == null) return;
        boolean bound = false;
        try {
            long generation = ++ipcGeneration;
            var mailbox = ipcOwner.activate(generation);
            CampaignLifecycleScript script = new CampaignLifecycleScript(generation, mailbox);
            sector.addTransientScript(script);
            ipcSector = sector;
            ipcScript = script;
            bound = true;
        } catch (NodeFailure stopped) { ipcOwner.suspend(); }
        finally { if (!bound) ipcOwner.suspend(); }
    }

    private void detachIpc() {
        if (ipcScript != null) ipcScript.close();
        if (ipcOwner != null) ipcOwner.suspend();
        if (ipcScript != null) ipcSector.removeTransientScript(ipcScript);
        ipcScript = null;
        ipcSector = null;
    }

    private void detachObserver() {
        if (callbackObserver == null) return;
        callbackObserver.close();
        callbackSector.removeTransientScript(callbackObserver);
        callbackObserver = null;
        callbackSector = null;
    }
}
