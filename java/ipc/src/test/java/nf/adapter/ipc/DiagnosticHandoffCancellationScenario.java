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

/** Characterizes real owner cancellation; no fake clock, test sleep or extra worker. */
final class DiagnosticHandoffCancellationScenario {
    private DiagnosticHandoffCancellationScenario() { }

    static void run(NodeLaunch launch, boolean interrupt) {
        if (Thread.currentThread().isInterrupted()) throw new AssertionError("clean owner interrupt state required");
        Path capture = launch.vault().getParent().resolve("handoff-cancellation.jfr");
        if (Files.exists(capture, LinkOption.NOFOLLOW_LINKS)) {
            throw new AssertionError("fresh owned cancellation recording required");
        }
        CancellationWindow current = new CancellationWindow(launch.executable(), interrupt);
        try {
            try (Recording recording = new Recording()) {
                recording.enable("nf.DiagnosticHandoffDelay").withoutStackTrace();
                recording.setMaxSize(4L * 1024 * 1024);
                recording.start();
                try {
                    try (OwnedNodeConnection unexpected = OwnedNodeConnection.open(
                            launch, current, new DiagnosticFaultPlan(5000))) {
                        unexpected.runtimeSession();
                        throw new AssertionError("cancelled handoff delay activated");
                    }
                } catch (NodeFailure refused) {
                    if (refused.code() != NodeFailure.Code.CANCELLED) {
                        throw new AssertionError("real handoff cancellation must refuse CANCELLED");
                    }
                    if (!current.requested || Thread.currentThread().isInterrupted() != interrupt) {
                        throw new AssertionError("actual cancellation and owner interrupt status required");
                    }
                } finally {
                    // Inspect first, then clear only the harness thread for JFR teardown.
                    if (interrupt) Thread.interrupted();
                }
                if (current.child == null || current.child.isAlive()) {
                    throw new AssertionError("exact cancelled owner child must be observed and reaped");
                }
                if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
                    throw new AssertionError("cancelled handoff publication survived cleanup");
                }
                recording.stop();
                recording.dump(capture);
            }
            List<RecordedEvent> events = DiagnosticHandoffProcessScenario.delayEvents(capture);
            if (events.size() != 1) throw new AssertionError("one real cancelled delay event required");
            RecordedEvent event = events.get(0);
            if (event.getLong("requestedMillis") != 5000 || !"CANCELLED".equals(event.getString("outcome"))
                    || event.getDuration().compareTo(Duration.ZERO) <= 0
                    || event.getDuration().compareTo(Duration.ofMillis(5000)) >= 0) {
                throw new AssertionError("real5000ms handoff must be cancelled during its actual delay");
            }
        } catch (IOException unavailable) {
            throw new AssertionError("bounded cancellation JFR capture unavailable");
        }
    }

    private static final class CancellationWindow implements BooleanSupplier {
        private final Path executable;
        private final boolean interrupt;
        private ProcessHandle child;
        private long observedAt;
        private boolean requested;

        CancellationWindow(Path executable, boolean interrupt) {
            this.executable = executable;
            this.interrupt = interrupt;
        }

        @Override public boolean getAsBoolean() {
            if (child == null) {
                List<ProcessHandle> matching = ProcessHandle.current().children().filter(candidate ->
                    candidate.info().command().map(command -> Path.of(command).equals(executable)).orElse(false))
                    .limit(2).toList();
                if (matching.size() > 1) throw new AssertionError("only one exact owned child admitted");
                if (!matching.isEmpty()) {
                    child = matching.get(0);
                    observedAt = System.nanoTime();
                }
            }
            if (child != null && System.nanoTime() - observedAt >= TimeUnit.MILLISECONDS.toNanos(100)) {
                requested = true;
                if (interrupt) Thread.currentThread().interrupt();
                else return false;
            }
            return true;
        }
    }
}
