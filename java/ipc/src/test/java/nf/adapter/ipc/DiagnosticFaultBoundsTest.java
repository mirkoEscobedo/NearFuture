package nf.adapter.ipc;

/** Closed fault-option admission at the public startup and value-policy boundaries. */
public final class DiagnosticFaultBoundsTest {
    private static final String ENABLED = "nf.ipc.diagnostic.enabled";
    private static final String DELAY = "nf.ipc.diagnostic.handoffDelayMillis";
    private DiagnosticFaultBoundsTest() { }

    public static void main(String[] args) {
        String enabled = System.getProperty(ENABLED);
        String delay = System.getProperty(DELAY);
        try {
            System.clearProperty(ENABLED);
            System.setProperty(DELAY, "5001");
            if (DiagnosticConfiguration.configured() != null) {
                throw new AssertionError("fault options activated diagnostics without explicit enable");
            }
        } finally {
            restore(ENABLED, enabled);
            restore(DELAY, delay);
        }
        DiagnosticFaultPlan maximum = new DiagnosticFaultPlan(5000);
        if (maximum.claimHandoffDelayMillis() != 5000) {
            throw new AssertionError("closed maximum handoff delay5000 was refused");
        }
        for (long value : new long[]{-1, 5001, Long.MIN_VALUE, Long.MAX_VALUE}) {
            try {
                new DiagnosticFaultPlan(value);
                throw new AssertionError("out-of-range worker handoff delay was admitted");
            } catch (NodeFailure refused) {
                if (refused.code() != NodeFailure.Code.INVALID) {
                    throw new AssertionError("worker delay admission must be INVALID");
                }
            }
        }
        System.out.println("Default-disabled and closed worker delay admission passed.");
    }

    private static void restore(String name, String value) {
        if (value == null) System.clearProperty(name);
        else System.setProperty(name, value);
    }
}
