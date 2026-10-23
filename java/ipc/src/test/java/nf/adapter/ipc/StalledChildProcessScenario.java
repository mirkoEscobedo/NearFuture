package nf.adapter.ipc;

import java.nio.file.Path;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;

/** Exercises the existing real startup and force-reap deadlines, with a pinned benign child. */
final class StalledChildProcessScenario {
    private StalledChildProcessScenario() { }
    static void run(NodeLaunch launch) {
        AtomicReference<ProcessHandle> owned = new AtomicReference<>();
        long start = System.nanoTime();
        NodeAttempt observation = new NodeAttempt();
        try (OwnedNodeConnection unexpected = OwnedNodeConnection.open(launch, () -> {
            ProcessHandle.current().children().filter(child -> child.info().command()
                .map(command -> Path.of(command).equals(launch.executable())).orElse(false))
                .findFirst().ifPresent(child -> owned.compareAndSet(null, child));
            return true;
        }, new DiagnosticFaultPlan(0), observation)) {
            unexpected.runtimeSession();
            throw new AssertionError("stalled child activated");
        } catch (NodeFailure failure) {
            if (failure.code() != NodeFailure.Code.TIMEOUT) throw new AssertionError("stalled child refusal");
        }
        ProcessHandle child = owned.get();
        if (child == null || child.isAlive()) throw new AssertionError("stalled owned child not reaped");
        NodeAttempt.Outcome outcome = observation.snapshot(false);
        if (outcome.phase() != NodeAttempt.Phase.HANDOFF || outcome.code() != NodeFailure.Code.TIMEOUT
                || outcome.detail() != NodeFailure.StartupDetail.NONE || !outcome.childCreated()
                || outcome.childPid() != child.pid() || !outcome.exitObserved() || outcome.exitStatus() == 0
                || outcome.cleanup() != NodeAttempt.Cleanup.FORCE_REAPED || outcome.cleanupFailure() != null) {
            throw new AssertionError("original startup timeout and actual forced reap must remain distinct facts");
        }
        // The fixture never exits voluntarily. The existing 15s startup + 8s grace
        // must elapse before destroyForcibly reaps that exact captured handle.
        if (System.nanoTime() - start < TimeUnit.SECONDS.toNanos(23)) {
            throw new AssertionError("force-reap escalation was not exercised");
        }
    }
}
