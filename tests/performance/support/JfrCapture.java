import java.nio.file.Path;
import java.time.Duration;
import jdk.jfr.Event;
import jdk.jfr.Name;
import jdk.jfr.Recording;
import jdk.jfr.StackTrace;

/** Bounded synthetic input only; no Starsector launch, attachment or game classes. */
public final class JfrCapture {
    @Name("nf.Frame")
    @StackTrace(false)
    static final class Frame extends Event {
        String traceId;
        String privateText;
        long frameId;
    }
    private JfrCapture() { }
    public static void main(String[] args) throws Exception {
        if (args.length != 2) throw new IllegalArgumentException("Expected output and trace ID");
        try (Recording recording = new Recording()) {
            recording.setMaxSize(8_388_608);
            recording.setMaxAge(Duration.ofSeconds(5));
            recording.enable(Frame.class).withoutStackTrace();
            recording.enable("jdk.ThreadSleep").withThreshold(Duration.ZERO).withoutStackTrace();
            recording.start();
            for (int i = 0; i < 3; i++) {
                Frame frame = new Frame();
                frame.traceId = args[1];
                frame.frameId = i;
                frame.privateText = "private-secret";
                frame.begin();
                Thread.sleep(2);
                frame.end();
                frame.commit();
            }
            recording.stop();
            recording.dump(Path.of(args[0]));
        }
    }
}
