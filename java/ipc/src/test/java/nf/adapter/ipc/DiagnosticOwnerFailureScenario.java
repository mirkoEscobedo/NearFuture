package nf.adapter.ipc;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.concurrent.TimeUnit;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordedEvent;

/** Real failed owner attempts retain closed causes even when no child was constructed. */
final class DiagnosticOwnerFailureScenario {
    private DiagnosticOwnerFailureScenario() { }

    static void run(NodeLaunch launch) throws InterruptedException {
        Path capture = launch.vault().getParent().resolve("owner-failure.jfr");
        try {
            try (Recording recording = new Recording()) {
                recording.enable("nf.OwnerAttempt").withoutStackTrace();
                recording.setMaxSize(4L * 1024 * 1024);
                recording.start();
                LifecycleOwner owner = new LifecycleOwner(launch);
                try {
                    LifecycleMailbox mailbox = owner.activate(401);
                    long until = System.nanoTime() + TimeUnit.SECONDS.toNanos(15);
                    while (mailbox.currentEpoch() != 6 && System.nanoTime() < until) Thread.sleep(10);
                    if (mailbox.currentEpoch() != 6) throw new AssertionError("three bounded startup attempts required");
                    if (!mailbox.active()) throw new AssertionError("failed start must preserve campaign binding");
                } finally {
                    owner.shutdown();
                    long until = System.nanoTime() + TimeUnit.SECONDS.toNanos(11);
                    while (!owner.isStopped() && System.nanoTime() < until) Thread.sleep(10);
                    if (!owner.isStopped()) throw new AssertionError("failed owner exceeded shutdown bound");
                }
                recording.stop();
                recording.dump(capture);
            }
            List<RecordedEvent> events = DiagnosticHandoffProcessScenario.events(capture, "nf.OwnerAttempt");
            if (events.size() != 3) {
                throw new AssertionError("real owner must retain three closed CHILD_START_IO outcomes; got" + events.size());
            }
            for (int index = 0; index < 3; index++) {
                RecordedEvent event = events.get(index);
                var stringFields = event.getFields().stream().filter(field -> field.getTypeName().equals("java.lang.String"))
                    .map(field -> field.getName()).collect(java.util.stream.Collectors.toSet());
                if (!stringFields.equals(java.util.Set.of("phase", "code", "detail", "cleanup", "cleanupFailure"))) {
                    throw new AssertionError("only the five closed string discriminators may be recorded");
                }
                if (event.getLong("generation") != 401 || event.getInt("attempt") != index + 1
                        || event.getLong("operationEpoch") != 1 + 2 * index
                        || event.getLong("terminalEpoch") != 2 + 2 * index
                        || !"CHILD_START".equals(event.getString("phase"))
                        || !"STARTUP".equals(event.getString("code"))
                        || !"CHILD_START_IO".equals(event.getString("detail"))
                        || event.getBoolean("childCreated") || event.getLong("childPid") != 0
                        || event.getBoolean("exitObserved") || event.getInt("exitStatus") != 0
                        || !"NO_CHILD".equals(event.getString("cleanup"))
                        || !"NONE".equals(event.getString("cleanupFailure"))) {
                    throw new AssertionError("closed failed-start facts or epoch correlation changed");
                }
                if (event.getThread() == null || !"nf-local-readonly-owner".equals(event.getThread().getJavaName())) {
                    throw new AssertionError("attempt observation must remain on the existing owner worker");
                }
            }
            if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))
                    || ProcessHandle.current().children().anyMatch(child -> child.info().command()
                        .map(command -> Path.of(command).equals(launch.executable())).orElse(false))) {
                throw new AssertionError("failed owner left a publication or child");
            }
        } catch (IOException unavailable) {
            throw new AssertionError("bounded owner failure recording unavailable");
        }
    }
}
