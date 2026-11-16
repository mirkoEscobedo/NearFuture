package nf.probe;

import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.security.SecureRandom;
import java.util.Arrays;
import java.util.concurrent.TimeUnit;
import java.util.function.BooleanSupplier;

/** Explicit launcher/host-side child owner. Not referenced by the base mod entry point.
 * Executable pinning, private-pipe polling and owned cleanup selectively adapt PR133's
 * OwnedNodeConnection. No vault, game identity, diagnostic faults or old lifecycle retries. */
public final class OwnedProbeHost implements AutoCloseable {
    /** Exact failed-start custody stays on the worker until that child is terminal. */
    static final class CleanupFailure extends IOException {
        private static final long serialVersionUID = 1L;
        private final transient Process child;
        CleanupFailure(Process child, Throwable cause) { super("Owned probe cleanup incomplete", cause); this.child = child; }
        long pid() { return child == null ? 0 : child.pid(); }
        void retry() throws IOException { if (child != null) stop(child); }
    }
    private final Process child;
    private final byte[] token;
    private final byte[] session;
    private final int port;
    private long generation;
    private boolean closed;
    private boolean forced;

    private OwnedProbeHost(Process child, byte[] token, byte[] ready) throws IOException {
        this.child = child;
        this.token = token;
        ByteBuffer r = ByteBuffer.wrap(ready);
        byte[] magic = new byte[8]; r.get(magic);
        port = Short.toUnsignedInt(r.getShort());
        session = new byte[16]; r.get(session);
        if (!Arrays.equals(magic, "NFMRDY01".getBytes(StandardCharsets.US_ASCII)) || port == 0
                || r.getLong() != 0 || Arrays.equals(session, new byte[16]))
            throw new IOException("Invalid owned probe readiness");
    }

    public static OwnedProbeHost start(Path executable, byte[] expectedSha256, int lifetimeMillis,
            ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        if (executable == null || !executable.isAbsolute() || expectedSha256 == null
                || expectedSha256.length != 32 || lifetimeMillis < 100 || lifetimeMillis > 60000)
            throw new IOException("Invalid owned probe launch");
        verifyExecutable(executable, expectedSha256, request, cancelled);
        byte[] token = new byte[32]; new SecureRandom().nextBytes(token);
        if (Arrays.equals(token, new byte[32])) throw new IOException("Random unavailable");
        Process child = null;
        boolean transferred = false;
        Throwable primary = null;
        try {
            requireCurrent(request, cancelled);
            child = new ProcessBuilder(executable.toString(), "serve-parent")
                    .redirectInput(ProcessBuilder.Redirect.PIPE).redirectOutput(ProcessBuilder.Redirect.PIPE)
                    .redirectError(ProcessBuilder.Redirect.DISCARD).start();
            byte[] bootstrap = ByteBuffer.allocate(48).put("NFMPAR01".getBytes(StandardCharsets.US_ASCII))
                    .put(token).putInt(lifetimeMillis).putInt(1).array();
            try { child.getOutputStream().write(bootstrap); child.getOutputStream().flush(); }
            finally { Arrays.fill(bootstrap, (byte)0); }
            OwnedProbeHost host = new OwnedProbeHost(child, token, read(child, 34, request, cancelled));
            transferred = true;
            return host;
        } catch (IOException | RuntimeException | Error failure) {
            primary = failure;
            throw failure;
        } finally {
            if (!transferred) {
                Arrays.fill(token, (byte)0);
                if (child != null) {
                    try { stop(child); }
                    catch (IOException | RuntimeException | LinkageError cleanup) {
                        CleanupFailure custody = new CleanupFailure(child, cleanup);
                        if (primary != null) custody.addSuppressed(primary);
                        throw custody;
                    }
                }
            }
        }
    }

    public void bind(ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        if (closed || request.generation() <= generation) throw new IOException("Invalid owned probe binding");
        requireCurrent(request, cancelled);
        child.getOutputStream().write(ByteBuffer.allocate(9).put((byte)1).putLong(request.generation()).array());
        child.getOutputStream().flush();
        byte[] ack = read(child, 32, request, cancelled);
        ByteBuffer a = ByteBuffer.wrap(ack);
        byte[] magic = new byte[8]; a.get(magic);
        byte[] acknowledgedSession = new byte[16]; a.get(acknowledgedSession);
        if (!Arrays.equals(magic, "NFMBND01".getBytes(StandardCharsets.US_ASCII))
                || !MessageDigest.isEqual(session, acknowledgedSession) || a.getLong() != request.generation())
            throw new IOException("Invalid owned probe binding acknowledgment");
        generation = request.generation();
    }
    public ModuleProbeClient connect(ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        if (closed || request.generation() != generation) throw new IOException("Stale owned probe binding");
        return ModuleProbeClient.connect(port, session, token, request, cancelled);
    }
    public long pid() { return child.pid(); }
    public boolean alive() { return child.isAlive(); }
    public boolean forcedCleanup() { return forced; }
    public byte[] serviceSession() { return session.clone(); }

    private static void verifyExecutable(Path executable, byte[] expected, ProbeRequest request,
            BooleanSupplier cancelled) throws IOException {
        if (!Files.isRegularFile(executable, LinkOption.NOFOLLOW_LINKS)) throw new IOException("Probe executable unavailable");
        MessageDigest digest;
        try { digest = MessageDigest.getInstance("SHA-256"); }
        catch (NoSuchAlgorithmException unavailable) { throw new IOException("Probe verification unavailable"); }
        try (InputStream input = Files.newInputStream(executable)) {
            byte[] buffer = new byte[65536]; long size = 0;
            for (int count; (count = input.read(buffer)) >= 0;) {
                requireCurrent(request, cancelled);
                size += count;
                if (size > 64L * 1024 * 1024) throw new IOException("Probe executable limit");
                digest.update(buffer, 0, count);
            }
        }
        if (!MessageDigest.isEqual(digest.digest(), expected)) throw new IOException("Probe executable identity rejected");
    }
    private static byte[] read(Process child, int bytes, ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        byte[] result = new byte[bytes]; int count = 0;
        InputStream input = child.getInputStream();
        while (count < bytes) {
            requireCurrent(request, cancelled);
            if (!child.isAlive()) throw new IOException("Owned probe exited");
            int available = input.available();
            if (available > 0) {
                int received = input.read(result, count, Math.min(available, bytes - count));
                if (received <= 0) throw new IOException("Owned probe pipe closed");
                count += received;
            } else {
                try { Thread.sleep(1); }
                catch (InterruptedException interrupted) { Thread.currentThread().interrupt(); throw new IOException("Probe interrupted"); }
            }
        }
        return result;
    }
    private static void requireCurrent(ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        if (request.expired() || cancelled.getAsBoolean()) throw new IOException("Probe cancelled or expired");
    }
    /** EOF requests ordinary lease revocation. Forced cleanup is recorded separately. */
    @Override public void close() throws IOException {
        if (closed && !child.isAlive()) return;
        closed = true;
        Arrays.fill(token, (byte)0);
        try { child.getOutputStream().close(); }
        finally { stop(child, () -> forced = true); }
    }
    private static boolean stop(Process child) throws IOException {
        return stop(child, () -> { });
    }
    static boolean stop(Process child, Runnable recordForce) throws IOException {
        boolean forced = false;
        boolean interrupted = Thread.interrupted();
        try {
            // Always attempt the exact owned process only, including on an interrupted owner.
            try { child.getOutputStream().close(); } catch (IOException ignored) { /* continue reap */ }
            try {
                if (!child.waitFor(2, TimeUnit.SECONDS)) {
                    forced = true; recordForce.run(); child.destroyForcibly();
                    if (!child.waitFor(2, TimeUnit.SECONDS)) throw new IOException("Owned probe cleanup incomplete");
                }
            } catch (InterruptedException interruption) {
                interrupted = true; forced = true; recordForce.run(); child.destroyForcibly();
                try {
                    if (!child.waitFor(2, TimeUnit.SECONDS)) throw new IOException("Owned probe cleanup incomplete");
                } catch (InterruptedException repeated) { throw new IOException("Owned probe cleanup interrupted"); }
            }
            return forced;
        } finally { if (interrupted) Thread.currentThread().interrupt(); }
    }
}
