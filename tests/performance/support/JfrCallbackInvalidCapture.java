import java.nio.file.Path;
import java.time.Duration;
import jdk.jfr.Event;
import jdk.jfr.Name;
import jdk.jfr.Recording;
import jdk.jfr.StackTrace;

/** Bounded malformed real JFR inputs for offline exporter rejection controls. */
public final class JfrCallbackInvalidCapture {
    @Name("nf.CampaignCallback")
    @StackTrace(false)
    static final class MissingTrace extends Event {
        String privateText;
    }
    @Name("nf.CampaignCallback")
    @StackTrace(false)
    static final class FrameClaim extends Event {
        String traceId;
        long frameId;
        String privateText;
    }
    @Name("nf.CampaignCallback")
    @StackTrace(false)
    static final class TraceOnly extends Event {
        String traceId;
        String privateText;
    }
    private JfrCallbackInvalidCapture() { }
    public static void main(String[] args) throws Exception {
        if (args.length != 3 || !args[1].matches("[a-f0-9]{32}")) throw new IllegalArgumentException();
        Event event;
        if (args[2].equals("missing-trace")) {
            MissingTrace value = new MissingTrace();
            value.privateText = "private-callback-secret";
            event = value;
        } else if (args[2].equals("frame-claim")) {
            FrameClaim value = new FrameClaim();
            value.traceId = args[1];
            value.frameId = 0;
            value.privateText = "private-callback-secret";
            event = value;
        } else if (args[2].equals("trace-mismatch")) {
            TraceOnly value = new TraceOnly();
            value.traceId = args[1];
            value.privateText = "private-callback-secret";
            event = value;
        } else throw new IllegalArgumentException();
        try (Recording recording = new Recording()) {
            recording.setMaxSize(8_388_608);
            recording.setMaxAge(Duration.ofSeconds(5));
            recording.enable(event.getClass()).withThreshold(Duration.ZERO).withoutStackTrace();
            recording.start();
            event.begin();
            Thread.sleep(2);
            event.end();
            event.commit();
            recording.stop();
            recording.dump(Path.of(args[0]));
        }
    }
}
