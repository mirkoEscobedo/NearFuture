package nf.adapter.probe;

/** Companion-local lifecycle surface; no worker, transport, optional mod or restricted JDK type. */
interface ProbeHooks {
    void resume();
    void suspend();
    String status();
}
