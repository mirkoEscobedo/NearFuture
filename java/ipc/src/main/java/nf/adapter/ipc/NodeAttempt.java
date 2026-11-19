package nf.adapter.ipc;

/** Worker-local facts; no throwable, process handle, path or peer bytes cross the outcome boundary. */
final class NodeAttempt {
    enum Phase { EXECUTABLE_VERIFY, CHILD_START, HANDOFF_DELAY, HANDOFF, CONTROL_AUTH, BULK_AUTH, RECONNECT, POLL }
    // EXIT_REAPED means observed exit without requesting force, not graceful application shutdown.
    enum Cleanup { NO_CHILD, EXIT_REAPED, FORCE_REAPED, NOT_REAPED }
    record Outcome(Phase phase, NodeFailure.Code code, NodeFailure.StartupDetail detail,
            boolean completed, boolean childCreated, long childPid, boolean exitObserved,
            int exitStatus, Cleanup cleanup, NodeFailure.Code cleanupFailure) { }

    private Phase phase = Phase.EXECUTABLE_VERIFY;
    private Phase failurePhase;
    private NodeFailure.Code code;
    private NodeFailure.StartupDetail detail = NodeFailure.StartupDetail.NONE;
    private boolean childCreated;
    private long childPid;
    private boolean exitObserved;
    private int exitStatus;
    private Cleanup cleanup = Cleanup.NO_CHILD;
    private NodeFailure.Code cleanupFailure;

    void phase(Phase value) { phase = value; }
    NodeFailure failure(NodeFailure value) {
        if (value.code() == NodeFailure.Code.CLEANUP) cleanupFailure = value.code();
        else if (code == null) {
            failurePhase = phase;
            code = value.code();
            detail = value.startupDetail();
        }
        return value;
    }
    void created(Process child) {
        childCreated = true;
        childPid = child.pid();
        cleanup = Cleanup.NOT_REAPED;
    }
    void reaped(Process child, boolean forced) {
        exitStatus = child.exitValue();
        exitObserved = true;
        cleanup = forced ? Cleanup.FORCE_REAPED : Cleanup.EXIT_REAPED;
    }
    Outcome snapshot(boolean completed) {
        return new Outcome(failurePhase == null ? phase : failurePhase, code, detail, completed,
            childCreated, childPid, exitObserved, exitStatus, cleanup, cleanupFailure);
    }

    static void record(long generation, int attempt, long operationEpoch, long terminalEpoch,
            Outcome outcome) {
        try {
            DiagnosticOwnerAttempt event = new DiagnosticOwnerAttempt();
            event.generation = generation;
            event.attempt = attempt;
            event.operationEpoch = operationEpoch;
            event.terminalEpoch = terminalEpoch;
            event.phase = outcome.phase().name();
            event.code = outcome.code() == null ? (outcome.completed() ? "NONE" : "UNKNOWN") : outcome.code().name();
            event.detail = outcome.detail().name();
            event.childCreated = outcome.childCreated();
            event.childPid = outcome.childPid();
            event.exitObserved = outcome.exitObserved();
            event.exitStatus = outcome.exitStatus();
            event.cleanup = outcome.cleanup().name();
            event.cleanupFailure = outcome.cleanupFailure() == null ? "NONE" : outcome.cleanupFailure().name();
            event.commit();
        } catch (RuntimeException | LinkageError unavailable) {
            // Optional telemetry never changes retry, cancellation or cleanup decisions.
        }
    }
}
