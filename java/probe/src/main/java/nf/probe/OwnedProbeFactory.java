package nf.probe;

import java.io.IOException;
import java.nio.file.Path;
import java.util.function.BooleanSupplier;

/** Worker-owned application service, independently rebound to campaign generations. One
 * connection per request; revocation closes that connection but retains the bounded child lease.
 * No retries per request. A failed cleanup permanently disables this factory. */
public final class OwnedProbeFactory implements ProbeTransport.Factory {
    public enum Phase { IDLE, START, BIND, AUTHENTICATE, APPLICATION, DETACH, STOPPED, CLEANUP_FAILED }
    private final Path executable;
    private final byte[] sha256;
    private OwnedProbeHost host;
    private long boundGeneration;
    private int bindings;
    private boolean disabled;
    private boolean cleanupFailed;
    private OwnedProbeHost.CleanupFailure failedStart;
    @FunctionalInterface
    interface Starter {
        OwnedProbeHost start(Path executable, byte[] sha256, ProbeRequest request, BooleanSupplier cancelled) throws IOException;
    }
    private final Starter starter;
    private volatile Phase phase = Phase.IDLE;
    private volatile long childPid;

    /** Immutable inputs only; file verification and process work occur later in open, on the worker. */
    public OwnedProbeFactory(Path executable, byte[] sha256) {
        this(executable, sha256, (path, pin, request, cancelled) -> OwnedProbeHost.start(path, pin, 60000, request, cancelled));
    }
    OwnedProbeFactory(Path executable, byte[] sha256, Starter starter) {
        if (executable == null || !executable.isAbsolute() || sha256 == null || sha256.length != 32)
            throw new IllegalArgumentException("Invalid probe launch");
        this.executable = executable;
        this.sha256 = sha256.clone();
        this.starter = starter;
    }
    public Phase phase() { return phase; }
    public long childPid() { return childPid; }

    @Override public ProbeTransport open(ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        if (disabled) throw new IOException("Probe owner unavailable");
        if (host != null && !host.alive()) release();
        if (host != null && bindings == 8 && request.generation() != boundGeneration) release();
        try {
        if (host == null) {
            phase = Phase.START;
            host = starter.start(executable, sha256, request, cancelled);
            childPid = host.pid();
            boundGeneration = 0;
            bindings = 0;
        }
        if (request.generation() != boundGeneration) {
            phase = Phase.BIND;
            host.bind(request, cancelled);
            boundGeneration = request.generation();
            bindings++;
        }
        phase = Phase.AUTHENTICATE;
        ModuleProbeClient client = host.connect(request, cancelled);
        return new ProbeTransport() {
            @Override public ProbeResult exchange(ProbeRequest value, BooleanSupplier revoked) throws IOException {
                phase = Phase.APPLICATION;
                return client.exchange(value, revoked);
            }
            @Override public void close() throws IOException { phase = Phase.DETACH; client.close(); }
        };
        } catch (IOException | RuntimeException | LinkageError failure) {
            if (failure instanceof OwnedProbeHost.CleanupFailure custody) {
                failedStart = custody; childPid = custody.pid();
                disabled = true; cleanupFailed = true; phase = Phase.CLEANUP_FAILED;
            }
            // A cancelled/partial parent bind has unknown application state. Never reuse it.
            try { release(); } catch (IOException | RuntimeException | LinkageError cleanup) { failure.addSuppressed(cleanup); }
            throw failure;
        }
    }
    private void release() throws IOException {
        if (host == null) return;
        OwnedProbeHost previous = host;
        try { previous.close(); }
        catch (IOException | RuntimeException | LinkageError unavailable) {
            disabled = true; cleanupFailed = true; phase = Phase.CLEANUP_FAILED; throw unavailable;
        }
        host = null;
    }
    @Override public void close() throws IOException {
        disabled = true;
        if (failedStart != null) failedStart.retry();
        release();
        if (cleanupFailed) { phase = Phase.CLEANUP_FAILED; throw new IOException("Owned probe cleanup previously failed"); }
        phase = Phase.STOPPED;
    }
}
