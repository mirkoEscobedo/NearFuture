package nf.adapter.ipc;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordedEvent;
import jdk.jfr.consumer.RecordingFile;

/** A dedicated harness process owns blocking work; no game callback is loaded. */
final class DiagnosticHandoffProcessScenario {
    private static final String EVENT = "nf.DiagnosticHandoffDelay";
    private static final long CAPTURE_LIMIT = 4L * 1024 * 1024;
    private DiagnosticHandoffProcessScenario() { }

    static void run(NodeLaunch launch) {
        Path capture = launch.vault().getParent().resolve("handoff-delay.jfr");
        if (Files.exists(capture, LinkOption.NOFOLLOW_LINKS)) {
            throw new AssertionError("fresh owned handoff recording required");
        }
        try {
            ProcessHandle child;
            try (Recording recording = new Recording()) {
                recording.enable(EVENT).withoutStackTrace();
                recording.setMaxSize(CAPTURE_LIMIT);
                recording.start();
                try (OwnedNodeConnection connection = OwnedNodeConnection.open(
                        launch, () -> true, new DiagnosticFaultPlan(750))) {
                    if (!connection.authenticatedBoth() || connection.runtimeSession() == 0) {
                        throw new AssertionError("real Rust roles must authenticate before delay evidence");
                    }
                    child = ProcessHandle.of(connection.childPid())
                        .orElseThrow(() -> new AssertionError("exact owned child handle required"));
                    if (!child.isAlive()) throw new AssertionError("owned child must be live before close");
                }
                if (child.isAlive()) throw new AssertionError("owned delay-test child was not reaped");
                if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
                    throw new AssertionError("owned delay-test publication survived close");
                }
                recording.stop();
                recording.dump(capture);
            }
            assertDelayEvent(capture);
        } catch (IOException unavailable) {
            throw new AssertionError("bounded handoff JFR capture unavailable");
        }
    }

    static List<RecordedEvent> delayEvents(Path capture) throws IOException {
        return events(capture, EVENT);
    }

    static List<RecordedEvent> events(Path capture, String selectedName) throws IOException {
        if (Files.size(capture) <= 0 || Files.size(capture) > CAPTURE_LIMIT) {
            throw new AssertionError("bounded handoff JFR capture size");
        }
        List<RecordedEvent> delays = new ArrayList<>();
        try (RecordingFile input = new RecordingFile(capture)) {
            int count = 0;
            while (input.hasMoreEvents()) {
                if (++count > 32) throw new AssertionError("handoff JFR event cap");
                RecordedEvent event = input.readEvent();
                if (event.getEventType().getName().equals(selectedName)) delays.add(event);
            }
        }
        return delays;
    }

    private static void assertDelayEvent(Path capture) throws IOException {
        List<RecordedEvent> delays = delayEvents(capture);
        if (delays.size() != 1) {
            throw new AssertionError("authenticated and reaped owner: real handoff delay event expected1, got" + delays.size());
        }
        RecordedEvent event = delays.get(0);
        if (event.getLong("requestedMillis") != 750 || !"COMPLETED".equals(event.getString("outcome"))) {
            throw new AssertionError("worker delay event must report actual requested750 and completion");
        }
        if (event.getDuration().compareTo(Duration.ofMillis(750)) < 0) {
            throw new AssertionError("real worker delay duration was shorter than750ms");
        }
    }
}
