package nf.adapter.probe;

import com.fs.starfarer.api.BaseModPlugin;

/** Separate, explicitly enabled diagnostic companion. The ordinary NearFuture plugin does
 * not reference this entrypoint or its driver. No static driver initializer or worker field. */
public final class ProbeModPlugin extends BaseModPlugin {
    private transient ProbeHooks hooks;
    private transient String startup = "DISABLED";

    @Override public void onApplicationLoad() {
        if (!"true".equals(System.getProperty("nf.probe.enabled"))) return;
        try { hooks = ProbeDriver.start(); startup = "STARTED"; }
        catch (RuntimeException | LinkageError unavailable) { startup = "DRIVER_UNAVAILABLE"; }
    }
    @Override public void onGameLoad(boolean newGame) { if (hooks != null) hooks.resume(); }
    @Override public void beforeGameSave() { if (hooks != null) hooks.suspend(); }
    @Override public void afterGameSave() { if (hooks != null) hooks.resume(); }
    @Override public void onGameSaveFailed() { if (hooks != null) hooks.resume(); }
    /** Closed diagnostic status only; no credentials, paths or peer text. No logging on callbacks. */
    public String diagnosticStatus() { return hooks == null ? startup : hooks.status(); }
}
