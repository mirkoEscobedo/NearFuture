import java.nio.file.Path;
import java.time.Duration;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordingFile;
import nf.adapter.ipc.LifecycleMailbox;
import nf.adapter.lifecycle.CampaignLifecycleScript;
import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Licensed API host, without native scheduling or serialization claims. */
public final class CampaignLifecycleControls {
    private CampaignLifecycleControls() { }
    public static void main(String[] args) throws Exception {
        Path output = Path.of(args[0]);
        LifecycleMailbox mailbox = new LifecycleMailbox(17);
        CampaignLifecycleScript script = new CampaignLifecycleScript(17, mailbox);
        try (Recording recording = new Recording()) {
            recording.enable("nf.LocalLifecycle").withThreshold(Duration.ZERO).withoutStackTrace();
            recording.start();
            long first = mailbox.beginConnection();
            mailbox.publish(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL, first, LifecycleMailbox.State.AUTHENTICATED);
            mailbox.publish(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK, first, LifecycleMailbox.State.AUTHENTICATED);
            script.advance(0);
            long second = mailbox.beginConnection();
            script.advance(0);
            mailbox.publish(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL, second, LifecycleMailbox.State.AUTHENTICATED);
            mailbox.publish(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK, second, LifecycleMailbox.State.AUTHENTICATED);
            script.advance(0);
            script.close();
            if (mailbox.active() || !script.isDone()) throw new AssertionError("save closure did not revoke value owner");
            script.advance(0);
            recording.stop(); recording.dump(output);
        }
        var events = RecordingFile.readAllEvents(output).stream()
            .filter(event -> event.getEventType().getName().equals("nf.LocalLifecycle")).toList();
        if (events.size() != 3) throw new AssertionError("exact three current lifecycle transitions required");
        String[] expected = {"AUTHENTICATED", "STARTING", "AUTHENTICATED"};
        long thread = Thread.currentThread().getId();
        for (int index = 0; index < 3; index++) {
            var event = events.get(index);
            if (!event.getString("control").equals(expected[index]) || !event.getString("bulk").equals(expected[index])
                || event.getLong("generation") != 17 || event.getLong("epoch") != (index == 0 ? 1 : 2)
                || event.getThread().getJavaThreadId() != thread) throw new AssertionError("current callback state mismatch");
        }
        LifecycleMailbox denied = new LifecycleMailbox(18);
        CampaignLifecycleScript wrongThread = new CampaignLifecycleScript(18, denied);
        Thread other = new Thread(() -> wrongThread.advance(0)); other.start(); other.join(2000);
        if (other.isAlive() || denied.active() || !wrongThread.isDone()) throw new AssertionError("wrong-thread callback was admitted");
        System.out.println("VALUE_CALLBACK_FENCES_OK");
    }
}
