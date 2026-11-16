package nf.probe;

import java.io.IOException;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicReference;

/** Application-lifetime daemon. Game callbacks publish/revoke values; every open/exchange/close
 * runs on this single worker. No native/process/storage capability is implied by this port. */
public final class ProbeService {
    private final ProbeTransport.Factory factory;
    private final AtomicReference<ProbeMailbox> requested = new AtomicReference<>();
    private volatile boolean running = true;
    private volatile boolean stopped;
    private volatile boolean cleanupFailed;
    private long lastGeneration;

    public ProbeService(ProbeTransport.Factory factory) {
        if (factory == null) throw new IllegalArgumentException("Missing probe transport");
        this.factory = factory;
        Thread worker = new Thread(this::run, "nf-probe-attach");
        worker.setDaemon(true);
        worker.start();
    }
    /** Single callback owner. Generations are never reused within this application lifetime. */
    public ProbeMailbox activate(long generation) {
        suspend();
        if (!running || generation <= lastGeneration) throw new IllegalStateException("Invalid campaign generation");
        lastGeneration = generation;
        ProbeMailbox next = new ProbeMailbox(generation);
        requested.set(next);
        if (!running) { next.revoke(); requested.compareAndSet(next, null); }
        return next;
    }
    public void suspend() {
        ProbeMailbox previous = requested.getAndSet(null);
        if (previous != null) previous.revoke();
    }
    public void shutdown() { running = false; suspend(); }
    public boolean isStopped() { return stopped; }
    public boolean cleanupFailed() { return cleanupFailed; }
    private boolean current(ProbeMailbox mailbox, ProbeRequest request) {
        return running && requested.get() == mailbox && mailbox.current(request);
    }
    private void run() {
        try {
            while (running) {
                ProbeMailbox mailbox = requested.get();
                ProbeRequest request = mailbox == null ? null : mailbox.take();
                if (request != null && current(mailbox, request)) exchange(mailbox, request);
                try { Thread.sleep(5); }
                catch (InterruptedException interrupted) { Thread.currentThread().interrupt(); shutdown(); }
            }
        } finally {
            shutdown();
            try { factory.close(); }
            catch (IOException | RuntimeException | LinkageError unavailable) { cleanupFailed = true; }
            finally { stopped = true; }
        }
    }
    private void exchange(ProbeMailbox mailbox, ProbeRequest request) {
        ProbeResult result;
        try (ProbeTransport transport = factory.open(request, () -> !current(mailbox, request))) {
            result = transport.exchange(request, () -> !current(mailbox, request));
            if (result == null || !request.matches(result)
                    || result.status() == ProbeResult.Status.REPLIED
                    && !Arrays.equals(request.challenge(), result.payload()))
                result = ProbeResult.failure(request, ProbeResult.Status.REJECTED);
        } catch (IOException | RuntimeException | LinkageError unavailable) {
            result = ProbeResult.failure(request, ProbeResult.Status.UNAVAILABLE);
        }
        mailbox.complete(request, result);
    }
}
