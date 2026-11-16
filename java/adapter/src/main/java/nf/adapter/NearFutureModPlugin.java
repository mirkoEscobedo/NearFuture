package nf.adapter;

import com.fs.starfarer.api.BaseModPlugin;
import com.fs.starfarer.api.Global;
import com.fs.starfarer.api.campaign.SectorAPI;
import nf.adapter.profiling.CampaignCallbackObserver;
import nf.contract.BuildIdentity;

/** Optional observer seam; no authority, recording ownership, or economic game mutation. */
public final class NearFutureModPlugin extends BaseModPlugin {
    private transient SectorAPI callbackSector;
    private transient CampaignCallbackObserver callbackObserver;

    public String syntheticBuildIdentity() {
        return BuildIdentity.description();
    }

    @Override
    public void onGameLoad(boolean newGame) {
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
        detachObserver();
    }

    private void detachObserver() {
        if (callbackObserver == null) return;
        callbackObserver.close();
        callbackSector.removeTransientScript(callbackObserver);
        callbackObserver = null;
        callbackSector = null;
    }
}
