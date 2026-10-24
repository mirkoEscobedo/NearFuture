package nf.adapter.ipc;

import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import org.nearfuture.ipc.v1.LocalEndpointRole;

/** One application-started background owner. Callback methods exchange only bounded values. */
public final class LifecycleOwner {
    private final NodeLaunch launch;
    private final DiagnosticFaultPlan faults;
    private final AtomicReference<LifecycleMailbox> requested = new AtomicReference<>();
    private volatile boolean running = true;
    private volatile boolean stopped;
    private long lastGeneration;
    private RetainedMailbox retained;

    private record RetainedMailbox(LifecycleMailbox mailbox, long epoch, LifecycleMailbox.State state) { }

    /** Application startup only: creates the sole daemon before campaign callbacks are installed. */
    public LifecycleOwner(NodeLaunch launch) {
        this(launch, new DiagnosticFaultPlan(0));
    }

    /** Startup injection only; the existing worker exclusively consumes the one-shot fault plan. */
    public LifecycleOwner(NodeLaunch launch, DiagnosticFaultPlan faults) {
        if (launch == null || faults == null) throw new NodeFailure(NodeFailure.Code.INVALID);
        this.launch = launch;
        this.faults = faults;
        Thread worker = new Thread(this::run, "nf-local-readonly-owner");
        worker.setDaemon(true);
        worker.start();
    }
    /** Single callback owner only. Revoke before publishing a new campaign binding. */
    public LifecycleMailbox activate(long generation) {
        suspend();
        if (!running || generation <= lastGeneration) throw new NodeFailure(NodeFailure.Code.INVALID);
        lastGeneration = generation;
        LifecycleMailbox mailbox = new LifecycleMailbox(generation);
        requested.set(mailbox);
        if (!running) { mailbox.revoke(); requested.compareAndSet(mailbox, null); }
        return mailbox;
    }
    /** Synchronous cancellation; waiting, socket closure and process reaping stay on the worker. */
    public void suspend() {
        LifecycleMailbox previous = requested.getAndSet(null);
        if (previous != null) previous.revoke();
    }
    public void shutdown() { running = false; suspend(); }
    public boolean isStopped() { return stopped; }
    private boolean current(LifecycleMailbox mailbox) {
        return running && mailbox.active() && requested.get() == mailbox;
    }
    private void run() {
        try {
            while (running) {
                releaseRevoked();
                LifecycleMailbox mailbox = requested.get();
                if (mailbox == null) { pause(); continue; }
                serve(mailbox);
                while (current(mailbox)) pause();
                releaseRevoked();
            }
        } finally {
            running = false;
            try { suspend(); }
            finally {
                try { releaseRevoked(); }
                finally { stopped = true; }
            }
        }
    }
    private void serve(LifecycleMailbox mailbox) {
        // Three finite child lifetimes per campaign binding, including failed starts.
        // A save/load creates a new binding; no callback performs a retry or process action.
        for (int attempt = 0; attempt < 3 && current(mailbox); attempt++) {
            try { connectionAttempt(mailbox); }
            catch (NodeFailure failure) {
                if (failure.code() == NodeFailure.Code.CLEANUP) {
                    // Never launch a successor if the exact owned child could not be reaped.
                    shutdown();
                    return;
                }
            }
        }
    }
    private void connectionAttempt(LifecycleMailbox mailbox) {
        long epoch = mailbox.beginConnection();
        publish(mailbox, epoch, LifecycleMailbox.State.STARTING);
        OwnedNodeConnection connection = null;
        try {
            connection = OwnedNodeConnection.open(launch, () -> current(mailbox), faults);
            if (retained == null && connection.authenticatedBoth() && current(mailbox)
                    && mailbox.isCurrent(epoch) && faults.claimStaleCapture()) {
                retained = new RetainedMailbox(mailbox, epoch, LifecycleMailbox.State.AUTHENTICATED);
            }
            publish(mailbox, epoch, LifecycleMailbox.State.AUTHENTICATED);
            long renew = System.nanoTime() + TimeUnit.SECONDS.toNanos(3);
            while (current(mailbox)) {
                connection.poll();
                if (System.nanoTime() - renew >= 0) {
                    epoch = mailbox.beginConnection();
                    if (epoch == 0) break;
                    publish(mailbox, epoch, LifecycleMailbox.State.STARTING);
                    connection.reconnect();
                    publish(mailbox, epoch, LifecycleMailbox.State.AUTHENTICATED);
                    renew = System.nanoTime() + TimeUnit.SECONDS.toNanos(3);
                }
                pause();
            }
        } finally {
            // Fence even unexpected failure BEFORE any bounded but blocking cleanup.
            try {
                long terminalEpoch = mailbox.beginConnection();
                publish(mailbox, terminalEpoch, LifecycleMailbox.State.UNAVAILABLE);
                releaseRevoked();
            } finally { if (connection != null) connection.close(); }
        }
    }

    private void releaseRevoked() {
        RetainedMailbox old = retained;
        if (old == null || old.mailbox().active()) return;
        retained = null;
        DiagnosticStaleMailbox event = new DiagnosticStaleMailbox();
        event.oldEpoch = old.epoch();
        event.state = old.state().name();
        event.oldActive = old.mailbox().active();
        event.controlAccepted = old.mailbox().publish(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL,
            old.epoch(), old.state());
        event.bulkAccepted = old.mailbox().publish(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK,
            old.epoch(), old.state());
        event.commit();
    }
    private static void publish(LifecycleMailbox mailbox, long epoch, LifecycleMailbox.State state) {
        mailbox.publish(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL, epoch, state);
        mailbox.publish(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK, epoch, state);
    }
    private void pause() {
        try { Thread.sleep(10); }
        catch (InterruptedException interrupted) { Thread.currentThread().interrupt(); shutdown(); }
    }
}
