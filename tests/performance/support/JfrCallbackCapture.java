import java.nio.file.Path;
import java.time.Duration;
import jdk.jfr.Event;
import jdk.jfr.Name;
import jdk.jfr.Recording;
import jdk.jfr.StackTrace;

/** Real bounded synthetic callback recording; no game classes or whole-frame event. */
public final class JfrCallbackCapture {
    @Name("nf.CampaignCallback")
    @StackTrace(false)
    static final class CampaignCallback extends Event {
        String traceId;
        String privateText;
    }
    private JfrCallbackCapture() { }
    public static void main(String[] args) throws Exception {
        if (args.length != 2 || !args[1].matches("[a-f0-9]{32}"))
            throw new IllegalArgumentException("Expected bounded output and hexadecimal trace ID");
        Thread owner = Thread.currentThread();
        String originalName = owner.getName();
        owner.setName("private-callback-secret");
        try (Recording recording = new Recording()) {
            recording.setMaxSize(8_388_608);
            recording.setMaxAge(Duration.ofSeconds(5));
            recording.enable(CampaignCallback.class).withThreshold(Duration.ZERO).withoutStackTrace();
            recording.start();
            for (int i = 0; i < 3; i++) {
                CampaignCallback callback = new CampaignCallback();
                callback.traceId = args[1];
                callback.privateText = "private-callback-secret";
                callback.begin();
                Thread.sleep(2);
                callback.end();
                callback.commit();
            }
            recording.stop();
            recording.dump(Path.of(args[0]));
        } finally {
            owner.setName(originalName);
        }
        System.out.println("{\"threadId\":" + owner.getId() + "}");
    }
}
