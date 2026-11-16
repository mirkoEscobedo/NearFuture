package nf.probe;

import java.util.concurrent.atomic.AtomicReference;

/** One callback producer/consumer and one worker; one pending request and one immutable reply.
 * Revocation follows the PR133 mailbox pattern: fence synchronously, clean up on the worker. */
public final class ProbeMailbox {
    private final long generation;
    private static final class Pending {
        final ProbeRequest request;
        final AtomicReference<ProbeResult> result = new AtomicReference<>();
        Pending(ProbeRequest request) { this.request = request; }
    }
    private final AtomicReference<Pending> outbound = new AtomicReference<>();
    private volatile Pending pending;
    private volatile boolean active = true;

    ProbeMailbox(long generation) { this.generation = generation; }
    public long generation() { return generation; }
    public boolean active() { return active; }

    /** Callback-only. One bounded allocation/copy; no I/O, crypto, waiting or worker creation. */
    public boolean submit(byte[] id, byte[] challenge, int timeoutMillis) {
        if (timeoutMillis < 1 || timeoutMillis > 5000) throw new IllegalArgumentException("Invalid probe deadline");
        if (!active || pending != null) return false;
        ProbeRequest request = new ProbeRequest(generation, id, challenge,
                System.nanoTime() + timeoutMillis * 1_000_000L);
        Pending next = new Pending(request);
        pending = next;
        outbound.set(next);
        if (!active) { cancel(); return false; }
        return true;
    }
    /** Callback-only. Returns at most one result; expiry wins over a late success. */
    public ProbeResult poll() {
        Pending current = pending;
        if (!active || current == null) return null;
        ProbeRequest request = current.request;
        ProbeResult result = current.result.getAndSet(null);
        if (request.expired()) result = ProbeResult.failure(request, ProbeResult.Status.DEADLINE);
        if (result == null || !request.matches(result) || !active) return null;
        pending = null;
        outbound.compareAndSet(current, null);
        return result;
    }
    public void cancel() { pending = null; outbound.set(null); }
    public void revoke() { active = false; cancel(); }
    ProbeRequest take() {
        Pending value = outbound.getAndSet(null);
        return active && value != null ? value.request : null;
    }
    boolean current(ProbeRequest request) {
        Pending value = pending;
        return active && value != null && value.request == request && !request.expired();
    }
    boolean complete(ProbeRequest request, ProbeResult result) {
        Pending value = pending;
        return active && value != null && value.request == request && !request.expired()
                && request.matches(result) && value.result.compareAndSet(null, result);
    }
}
