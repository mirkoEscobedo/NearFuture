package nf.performance;

import java.nio.file.Path;
import java.time.Duration;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordingFile;
import org.lwjgl.opengl.Display;

public final class PresentationFixture {
    private PresentationFixture() { }
    public static void main(String[] args) throws Exception {
        Path output = Path.of(args[0]);
        if (args[1].equals("admitted") && (!"0123456789abcdef0123456789abcdef".equals(System.getProperty("nf.profiling.traceId"))
            || !"false".equals(System.getProperty("nf.profiling.callback.enabled")))) {
            throw new AssertionError("fixed-launcher environment correlation was not applied");
        }
        try (Recording recording = new Recording()) {
            recording.setName("NF100");
            recording.enable("jdk.JVMInformation").with("period", "beginChunk");
            recording.enable("nf.PresentInterval").withThreshold(Duration.ZERO).withoutStackTrace();
            recording.start();
            for (int call = 0; call < 4; call++) { Display.swapBuffers(); Thread.sleep(5); }
            recording.stop(); recording.dump(output);
        }
        if (Display.swaps != 4) throw new AssertionError("instrumentation changed presentation effects");
        var events = RecordingFile.readAllEvents(output).stream()
            .filter(event -> event.getEventType().getName().equals("nf.PresentInterval")).toList();
        if (!args[1].equals("admitted")) {
            if (!events.isEmpty()) throw new AssertionError("unadmitted probe emitted presentation intervals");
            System.out.println("PRESENTATION_UNOBSERVED_OK");
            return;
        }
        if (events.size() != 3) throw new AssertionError("four actual presentations require three measured intervals");
        long thread = Thread.currentThread().getId();
        for (int index = 0; index < 3; index++) {
            var event = events.get(index);
            if (event.getLong("frameId") != index + 1 || event.getLong("intervalNanos") <= 0
                || event.getThread().getJavaThreadId() != thread
                || !event.getString("traceId").equals("0123456789abcdef0123456789abcdef")) {
                throw new AssertionError("literal presentation provenance mismatch");
            }
        }
        System.out.println("PRESENTATION_INTERVALS_OK");
    }
}
