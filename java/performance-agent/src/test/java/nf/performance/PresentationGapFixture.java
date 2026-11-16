package nf.performance;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordingFile;
import org.lwjgl.opengl.Display;

/** A stopped recording cannot be stitched to a later recording under the same JVM trace. */
public final class PresentationGapFixture {
    private PresentationGapFixture() { }
    private static Recording recording() {
        Recording recording = new Recording();
        recording.setName("NF100");
        recording.enable("nf.PresentInterval").withThreshold(Duration.ZERO).withoutStackTrace();
        recording.enable("nf.FrameProbeRejected").withThreshold(Duration.ZERO).withoutStackTrace();
        return recording;
    }
    public static void main(String[] args) throws Exception {
        Path first = Files.createTempFile("nf-presentation-before-gap-", ".jfr");
        try {
            try (Recording recording = recording()) {
                recording.start(); Display.swapBuffers(); Thread.sleep(5); Display.swapBuffers();
                recording.stop(); recording.dump(first);
            }
            if (RecordingFile.readAllEvents(first).stream()
                .filter(event -> event.getEventType().getName().equals("nf.PresentInterval")).count() != 1) {
                throw new AssertionError("the gap fixture must first observe a genuine interval");
            }
            if (!args[1].equals("silent-gap")) Display.swapBuffers();
            Path second = Path.of(args[0]);
            try (Recording recording = recording()) {
                recording.start(); Display.swapBuffers(); Thread.sleep(5); Display.swapBuffers();
                recording.stop(); recording.dump(second);
            }
            var events = RecordingFile.readAllEvents(second);
            if (events.stream().anyMatch(event -> event.getEventType().getName().equals("nf.PresentInterval"))) {
                throw new AssertionError("recording gap revived presentation observation");
            }
            var rejected = events.stream().filter(event -> event.getEventType().getName().equals("nf.FrameProbeRejected")).toList();
            if (rejected.size() != 1 || rejected.get(0).getInt("reason") != 4) {
                throw new AssertionError("recording gap must retain one explicit rejection");
            }
            if (Display.swaps != (args[1].equals("silent-gap") ? 4 : 5)) throw new AssertionError("gap handling changed callee effects");
        } finally { Files.deleteIfExists(first); }
        System.out.println("PRESENTATION_GAP_REVOKED_OK");
    }
}
