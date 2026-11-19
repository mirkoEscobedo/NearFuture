package nf.adapter.ipc;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.time.Duration;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.function.BooleanSupplier;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordedEvent;

/** Uses the existing real stalled executable and a real cancellation guard, never a fake clock. */
final class DiagnosticStartupBudgetScenario {
    private DiagnosticStartupBudgetScenario() { }

    static void run(NodeLaunch launch) {
        Path capture = launch.vault().getParent().resolve("startup-budget.jfr");
        if (Files.exists(capture, LinkOption.NOFOLLOW_LINKS)) {
            throw new AssertionError("fresh owned startup-budget recording required");
        }
        try {
            try (Recording recording = new Recording()) {
                recording.enable("nf.DiagnosticHandoffDelay").withoutStackTrace();
                recording.setMaxSize(4L * 1024 * 1024);
                recording.start();
                BudgetWindow current = new BudgetWindow(launch.executable());
                try (OwnedNodeConnection unexpected = OwnedNodeConnection.open(
                        launch, current, new DiagnosticFaultPlan(5000))) {
                    unexpected.runtimeSession();
                    throw new AssertionError("stalled diagnostic child activated");
                } catch (NodeFailure refused) {
                    if (refused.code() != NodeFailure.Code.TIMEOUT || current.watchdogFired) {
                        throw new AssertionError("5000ms handoff must consume original15s startup budget, not reach16s cancellation");
                    }
                }
                if (current.child == null || current.child.isAlive()) {
                    throw new AssertionError("exact stalled budget child must be observed and forcibly reaped");
                }
                if (System.nanoTime() - current.started < TimeUnit.SECONDS.toNanos(23)) {
                    throw new AssertionError("original15s startup plus8s forced-cleanup grace was not exercised");
                }
                if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
                    throw new AssertionError("stalled budget publication survived cleanup");
                }
                recording.stop();
                recording.dump(capture);
            }
            List<RecordedEvent> events = DiagnosticHandoffProcessScenario.delayEvents(capture);
            if (events.size() != 1) throw new AssertionError("one actual injected5000ms delay event required");
            RecordedEvent event = events.get(0);
            if (event.getLong("requestedMillis") != 5000 || !"COMPLETED".equals(event.getString("outcome"))
                    || event.getDuration().compareTo(Duration.ofMillis(5000)) < 0) {
                throw new AssertionError("actual5000ms handoff delay must complete inside original startup budget");
            }
        } catch (IOException unavailable) {
            throw new AssertionError("bounded startup-budget JFR capture unavailable");
        }
    }

    private static final class BudgetWindow implements BooleanSupplier {
        private final Path executable;
        private final long started = System.nanoTime();
        private ProcessHandle child;
        private boolean watchdogFired;
        BudgetWindow(Path executable) { this.executable = executable; }

        @Override public boolean getAsBoolean() {
            if (child == null) {
                List<ProcessHandle> children = ProcessHandle.current().children().filter(candidate ->
                    candidate.info().command().map(command -> Path.of(command).equals(executable)).orElse(false))
                    .limit(2).toList();
                if (children.size() > 1) throw new AssertionError("only one exact stalled budget child admitted");
                if (!children.isEmpty()) child = children.get(0);
            }
            if (System.nanoTime() - started >= TimeUnit.SECONDS.toNanos(16)) {
                watchdogFired = true;
                return false;
            }
            return true;
        }
    }
}
