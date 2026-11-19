package nf.adapter.ipc;

/** Worker-owned one-shot value policy; never waits, starts work or performs I/O. */
public final class DiagnosticFaultPlan {
    private long pendingDelayMillis;
    private boolean pendingStaleCapture;

    public DiagnosticFaultPlan(long handoffDelayMillis) {
        this(handoffDelayMillis, false);
    }

    public DiagnosticFaultPlan(long handoffDelayMillis, boolean retainStaleOnce) {
        if (handoffDelayMillis < 0 || handoffDelayMillis > 5000) {
            throw new NodeFailure(NodeFailure.Code.INVALID);
        }
        pendingDelayMillis = handoffDelayMillis;
        pendingStaleCapture = retainStaleOnce;
    }

    public long claimHandoffDelayMillis() {
        long claimed = pendingDelayMillis;
        pendingDelayMillis = 0;
        return claimed;
    }

    boolean claimStaleCapture() {
        boolean claimed = pendingStaleCapture;
        pendingStaleCapture = false;
        return claimed;
    }
}
