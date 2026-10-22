import java.nio.file.Path;
import java.time.Duration;
import jdk.jfr.Event;
import jdk.jfr.Name;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordingFile;

/** Literal marker fields exercise offline export, without a renderer or native evidence. */
public final class JfrPresentationCapture {
    private JfrPresentationCapture() { }
    @Name("nf.PresentInterval")
    public static final class Presentation extends Event {
        public String traceId;
        public long frameId;
        public long intervalNanos;
        public long clockUncertaintyNanos;
        public String privateValue = "private-marker-secret";
    }
    @Name("nf.FrameProbeRejected")
    public static final class Rejected extends Event { public int reason = 4; }
    @Name("jdk.DataLoss")
    public static final class Loss extends Event { public long amount = 1; }
    public static void main(String[] args) throws Exception {
        Path path = Path.of(args[0]);
        String mode = args[2];
        try (Recording recording = new Recording()) {
            recording.enable("nf.PresentInterval").withThreshold(Duration.ZERO).withoutStackTrace();
            recording.enable("nf.FrameProbeRejected").withThreshold(Duration.ZERO).withoutStackTrace();
            recording.enable("jdk.DataLoss").withThreshold(Duration.ZERO).withoutStackTrace();
            recording.start();
            Presentation first = new Presentation();
            first.traceId = args[1]; first.frameId = 1;
            first.intervalNanos = mode.equals("zero") ? 0 : 12_345_000;
            first.clockUncertaintyNanos = mode.equals("negative-uncertainty") ? -1 : 2000;
            first.begin(); first.commit();
            if (mode.equals("probe-rejected")) new Rejected().commit();
            if (mode.equals("data-loss")) new Loss().commit();
            if (mode.equals("valid")) {
                Presentation second = new Presentation();
                second.traceId = args[1]; second.frameId = 2;
                second.intervalNanos = 3_000_000_000L; second.clockUncertaintyNanos = 500;
                second.begin(); second.commit();
            }
            recording.stop(); recording.dump(path);
        }
        var events = RecordingFile.readAllEvents(path).stream()
            .filter(event -> event.getEventType().getName().equals("nf.PresentInterval")).toList();
        System.out.print("[");
        for (int index = 0; index < events.size(); index++) {
            if (index > 0) System.out.print(",");
            var instant = events.get(index).getStartTime();
            System.out.print(instant.getEpochSecond() * 1_000_000 + instant.getNano() / 1000);
        }
        System.out.println("]");
    }
}
