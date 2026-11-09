package nf.performance;

import java.nio.file.Path;
import java.time.Duration;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordingFile;
import org.lwjgl.opengl.Display;

/** A second presentation thread invalidates the whole recording instead of mixing cadences. */
public final class PresentationThreadFixture {
    private PresentationThreadFixture() { }
    public static void main(String[] args) throws Exception {
        Path output = Path.of(args[0]);
        try (Recording recording = new Recording()) {
            recording.setName("NF100");
            recording.enable("nf.PresentInterval").withThreshold(Duration.ZERO).withoutStackTrace();
            recording.enable("nf.FrameProbeRejected").withThreshold(Duration.ZERO).withoutStackTrace();
            recording.start();
            Display.swapBuffers(); Thread.sleep(5); Display.swapBuffers();
            Thread foreign = new Thread(Display::swapBuffers);
            foreign.start(); foreign.join(2000);
            if (foreign.isAlive()) throw new AssertionError("foreign presentation did not return");
            Display.swapBuffers(); Display.swapBuffers();
            recording.stop(); recording.dump(output);
        }
        var events = RecordingFile.readAllEvents(output);
        long count = events.stream().filter(event -> event.getEventType().getName().equals("nf.PresentInterval")).count();
        var rejected = events.stream().filter(event -> event.getEventType().getName().equals("nf.FrameProbeRejected")).toList();
        if (count != 1 || rejected.size() != 1 || rejected.get(0).getInt("reason") != 2) {
            throw new AssertionError("mixed rendering threads were not explicitly rejected");
        }
        if (Display.swaps != 5) throw new AssertionError("thread rejection changed presentation effects");
        System.out.println("PRESENTATION_THREAD_REVOKED_OK");
    }
}
