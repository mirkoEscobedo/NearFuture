package nf.adapter.ipc;

/** Independent value-only oracle; claims do not sleep, start a worker or perform I/O. */
public final class DiagnosticHandoffDelayTest {
    private DiagnosticHandoffDelayTest() { }

    public static void main(String[] args) {
        DiagnosticFaultPlan disabled = new DiagnosticFaultPlan(0);
        if (disabled.claimHandoffDelayMillis() != 0) {
            throw new AssertionError("disabled worker handoff delay must remain0");
        }
        DiagnosticFaultPlan delayed = new DiagnosticFaultPlan(750);
        long first = delayed.claimHandoffDelayMillis();
        if (first != 750) {
            throw new AssertionError("first worker handoff delay expected750, got" + first);
        }
        if (delayed.claimHandoffDelayMillis() != 0 || delayed.claimHandoffDelayMillis() != 0) {
            throw new AssertionError("worker handoff delay was offered more than once");
        }
        System.out.println("One-shot worker handoff delay claims passed; no worker/native claim.");
    }
}
