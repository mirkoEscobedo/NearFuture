package nf.adapter.lifecycle;

import com.fs.starfarer.api.EveryFrameScript;
import nf.adapter.ipc.LifecycleMailbox;
import org.nearfuture.ipc.v1.LocalEndpointRole;
import jdk.jfr.Category;
import jdk.jfr.Event;
import jdk.jfr.Label;
import jdk.jfr.Name;
import jdk.jfr.StackTrace;

/** Transient value consumer; never owns a service, process, token, socket or worker. */
public final class CampaignLifecycleScript implements EveryFrameScript {
    private final long generation;
    private final LifecycleMailbox mailbox;
    private final Thread callbackThread = Thread.currentThread();
    private long lastEpoch;
    private LifecycleMailbox.State control = LifecycleMailbox.State.STARTING;
    private LifecycleMailbox.State bulk = LifecycleMailbox.State.STARTING;
    private volatile boolean closed;

    public CampaignLifecycleScript(long generation, LifecycleMailbox mailbox) {
        if (generation <= 0 || mailbox == null) throw new IllegalArgumentException("Invalid value binding");
        this.generation = generation;
        this.mailbox = mailbox;
    }
    @Override public boolean isDone() { return closed || !mailbox.active(); }
    @Override public boolean runWhilePaused() { return true; }
    @Override public void advance(float amount) {
        if (isDone()) return;
        if (Thread.currentThread() != callbackThread) { close(); return; }
        long epoch = mailbox.currentEpoch();
        if (epoch == 0) return;
        boolean changed = epoch != lastEpoch;
        LifecycleMailbox.State nextControl = changed ? LifecycleMailbox.State.STARTING : control;
        LifecycleMailbox.State nextBulk = changed ? LifecycleMailbox.State.STARTING : bulk;
        var receivedControl = mailbox.poll(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL);
        var receivedBulk = mailbox.poll(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK);
        if (!mailbox.isCurrent(epoch)) return;
        if (receivedControl != null) nextControl = receivedControl;
        if (receivedBulk != null) nextBulk = receivedBulk;
        changed |= nextControl != control || nextBulk != bulk;
        lastEpoch = epoch; control = nextControl; bulk = nextBulk;
        if (changed) {
            LifecycleEvent event = new LifecycleEvent();
            if (event.isEnabled()) {
                event.generation = generation; event.epoch = epoch;
                event.control = control.name(); event.bulk = bulk.name();
                event.commit();
            }
        }
    }
    public void close() { mailbox.revoke(); closed = true; }

    @Name("nf.LocalLifecycle")
    @Label("NearFuture read-only local lifecycle")
    @Category("NearFuture")
    @StackTrace(false)
    private static final class LifecycleEvent extends Event {
        public long generation;
        public long epoch;
        public String control;
        public String bulk;
    }
}
