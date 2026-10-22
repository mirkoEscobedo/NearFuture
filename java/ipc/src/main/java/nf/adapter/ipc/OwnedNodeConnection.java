package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.security.SecureRandom;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.function.BooleanSupplier;
import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Blocking lifecycle shell, exclusively owned by a background thread. Never call from a game callback. */
public final class OwnedNodeConnection implements AutoCloseable {
    private static final long STARTUP_NANOS = TimeUnit.SECONDS.toNanos(15);
    private final Process child;
    private final OutputStream parent;
    private final InputStream privateOutput;
    private final BooleanSupplier current;
    private ParentHandoff handoff;
    private BackgroundChannel control;
    private BackgroundChannel bulk;
    private boolean authenticated;
    private boolean closed;

    private OwnedNodeConnection(Process child, BooleanSupplier current) {
        this.child = child;
        this.parent = child.getOutputStream();
        this.privateOutput = child.getInputStream();
        this.current = current;
    }

    public static OwnedNodeConnection open(NodeLaunch launch, BooleanSupplier current) {
        return open(launch, current, new DiagnosticFaultPlan(0));
    }

    /** Opt-in faults run only on this blocking owner and consume its original startup budget. */
    public static OwnedNodeConnection open(NodeLaunch launch, BooleanSupplier current, DiagnosticFaultPlan faults) {
        if (launch == null || current == null || faults == null) throw new NodeFailure(NodeFailure.Code.INVALID);
        long deadline = System.nanoTime() + STARTUP_NANOS;
        OwnedNodeConnection owned = null;
        Process spawned = null;
        boolean transferred = false;
        try {
            verifyExecutable(launch, current, deadline);
            requireCurrent(current, deadline);
            var scope = launch.scope();
            List<String> command = List.of(launch.executable().toString(), "serve-parent",
                launch.vault().toString(), launch.saves().toString(), launch.publication(),
                hex(scope.universe()), hex(scope.history()), hex(scope.ruleset()), hex(scope.content()),
                hex(scope.principal().getAccountId().getValue()), hex(scope.principal().getDeviceId().getValue()),
                Long.toString(launch.durationMillis()));
            spawned = new ProcessBuilder(command)
                .redirectInput(ProcessBuilder.Redirect.PIPE).redirectOutput(ProcessBuilder.Redirect.PIPE)
                .redirectError(ProcessBuilder.Redirect.DISCARD).start();
            owned = new OwnedNodeConnection(spawned, current);
            owned.delayHandoff(faults, deadline);
            owned.activate(launch.scope(), deadline);
            transferred = true;
            return owned;
        } catch (NodeFailure failure) {
            throw failure;
        } catch (AuthFailure rejected) {
            throw new NodeFailure(NodeFailure.Code.AUTHENTICATION);
        } catch (IOException | NoSuchAlgorithmException failure) {
            throw new NodeFailure(NodeFailure.Code.STARTUP);
        } catch (RuntimeException failure) {
            throw new NodeFailure(NodeFailure.Code.PROTOCOL);
        } finally {
            // Includes linkage/owner Errors: successful construction alone transfers
            // custody to the caller. A failed factory never abandons its child.
            if (!transferred) {
                if (owned != null) owned.close();
                else if (spawned != null) stopChild(spawned);
            }
        }
    }

    private static String hex(ByteString bytes) { return HexFormat.of().formatHex(bytes.toByteArray()); }

    private void delayHandoff(DiagnosticFaultPlan faults, long deadline) {
        requireAlive(deadline);
        long millis = faults.claimHandoffDelayMillis();
        if (millis == 0) return;
        DiagnosticHandoffDelay event = new DiagnosticHandoffDelay();
        event.requestedMillis = millis;
        event.outcome = "FAILED";
        event.begin();
        try {
            long until = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(millis);
            while (System.nanoTime() - until < 0) {
                requireAlive(deadline);
                pause();
            }
            requireAlive(deadline);
            event.outcome = "COMPLETED";
        } catch (NodeFailure refused) {
            event.outcome = refused.code().name();
            throw refused;
        } finally {
            event.end();
            event.commit();
        }
    }

    private static void verifyExecutable(NodeLaunch launch, BooleanSupplier current, long deadline)
            throws IOException, NoSuchAlgorithmException {
        if (!Files.isRegularFile(launch.executable(), LinkOption.NOFOLLOW_LINKS)) throw new NodeFailure(NodeFailure.Code.PIN);
        MessageDigest digest = MessageDigest.getInstance("SHA-256");
        try (InputStream input = Files.newInputStream(launch.executable())) {
            byte[] buffer = new byte[65536];
            long count = 0;
            for (int read; (read = input.read(buffer)) >= 0;) {
                requireCurrent(current, deadline);
                count += read;
                if (count > 64L * 1024 * 1024) throw new NodeFailure(NodeFailure.Code.PIN);
                digest.update(buffer, 0, read);
            }
        }
        if (!MessageDigest.isEqual(digest.digest(), launch.executableSha256().toByteArray())) {
            throw new NodeFailure(NodeFailure.Code.PIN);
        }
    }

    private void activate(ParentHandoff.Scope scope, long deadline) throws IOException {
        byte[] nonce = new byte[32];
        byte[] packet = new byte[ParentHandoff.BYTES];
        try {
            new SecureRandom().nextBytes(nonce);
            parent.write("NF-PARENT-1\0".getBytes(java.nio.charset.StandardCharsets.US_ASCII));
            parent.write(nonce); parent.flush();
            int count = 0;
            while (count < packet.length) {
                requireAlive(deadline);
                int available = privateOutput.available();
                if (available == 0) { pause(); continue; }
                int read = privateOutput.read(packet, count, Math.min(available, packet.length - count));
                if (read <= 0) throw new NodeFailure(NodeFailure.Code.PROTOCOL);
                count += read;
            }
            rejectTrailingOutput();
            handoff = ParentHandoff.decode(packet, nonce, scope);
        } finally { Arrays.fill(packet, (byte)0); Arrays.fill(nonce, (byte)0); }
        control = authenticate(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL, deadline);
        bulk = authenticate(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK, deadline);
        requireAlive(deadline);
        rejectTrailingOutput();
        authenticated = true;
    }

    private BackgroundChannel authenticate(LocalEndpointRole role, long deadline) throws IOException {
        AuthContext context = handoff.context(role);
        BackgroundChannel channel = new BackgroundChannel(new InetSocketAddress(
            InetAddress.getByAddress(new byte[]{127, 0, 0, 1}), context.port()),
            Math.min(4096, context.localLimits().getControlFrameBytes()));
        boolean active = false;
        try (MutualAuthClient client = handoff.client(role)) {
            if (!channel.send(client.hello().toByteArray())) throw new NodeFailure(NodeFailure.Code.PROTOCOL);
            byte[] challenge = receive(channel, deadline);
            try {
                if (!channel.send(client.respond(challenge).toByteArray())) throw new NodeFailure(NodeFailure.Code.PROTOCOL);
            } finally { Arrays.fill(challenge, (byte)0); }
            byte[] accepted = receive(channel, deadline);
            try { client.finish(accepted); } finally { Arrays.fill(accepted, (byte)0); }
            active = client.isActive();
            if (!active) throw new NodeFailure(NodeFailure.Code.AUTHENTICATION);
            return channel;
        } finally { if (!active) channel.close(); }
    }

    private byte[] receive(BackgroundChannel channel, long deadline) throws IOException {
        while (true) {
            requireAlive(deadline);
            rejectTrailingOutput();
            byte[] message = channel.poll();
            if (message != null) return message;
            pause();
        }
    }
    private void rejectTrailingOutput() throws IOException {
        if (privateOutput.available() != 0) throw new NodeFailure(NodeFailure.Code.PROTOCOL);
    }
    private static void requireCurrent(BooleanSupplier current, long deadline) {
        if (Thread.currentThread().isInterrupted() || !current.getAsBoolean()) throw new NodeFailure(NodeFailure.Code.CANCELLED);
        if (System.nanoTime() - deadline >= 0) throw new NodeFailure(NodeFailure.Code.TIMEOUT);
    }
    private void requireAlive(long deadline) {
        requireCurrent(current, deadline);
        if (!child.isAlive()) throw new NodeFailure(NodeFailure.Code.CHILD_EXIT);
    }
    private static void pause() {
        try { Thread.sleep(1); }
        catch (InterruptedException interrupted) { Thread.currentThread().interrupt(); throw new NodeFailure(NodeFailure.Code.CANCELLED); }
    }

    public boolean authenticatedBoth() { return authenticated && !closed && child.isAlive() && current.getAsBoolean(); }
    public long runtimeSession() {
        if (closed || handoff == null) throw new NodeFailure(NodeFailure.Code.CHILD_EXIT);
        return handoff.context(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL).runtimeSession();
    }
    public long childPid() { return child.pid(); }

    /** Background only. The caller must advance its callback epoch before this disconnect. */
    public void reconnect() {
        authenticated = false;
        try {
            if (closed || handoff == null) throw new NodeFailure(NodeFailure.Code.CHILD_EXIT);
            if (control != null) { control.close(); control = null; }
            if (bulk != null) { bulk.close(); bulk = null; }
            long deadline = System.nanoTime() + STARTUP_NANOS;
            requireAlive(deadline);
            control = authenticate(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL, deadline);
            bulk = authenticate(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK, deadline);
            requireAlive(deadline);
            rejectTrailingOutput();
            authenticated = true;
        } catch (IOException disconnected) { throw new NodeFailure(NodeFailure.Code.DISCONNECTED); }
    }

    /** One background turn. There are no application requests in this diagnostic checkpoint. */
    public void poll() {
        if (closed) throw new NodeFailure(NodeFailure.Code.CHILD_EXIT);
        requireAlive(System.nanoTime() + TimeUnit.SECONDS.toNanos(1));
        try {
            rejectTrailingOutput();
            if (control.poll() != null || bulk.poll() != null) throw new NodeFailure(NodeFailure.Code.PROTOCOL);
        } catch (IOException disconnected) { throw new NodeFailure(NodeFailure.Code.DISCONNECTED); }
    }

    @Override public void close() {
        if (closed) return;
        closed = true; authenticated = false;
        try { parent.close(); } catch (IOException ignored) { }
        if (control != null) try { control.close(); } catch (IOException ignored) { }
        if (bulk != null) try { bulk.close(); } catch (IOException ignored) { }
        if (handoff != null) handoff.close();
        stopChild(child);
    }

    private static void stopChild(Process child) {
        boolean interrupted = Thread.interrupted();
        try {
            try { child.getOutputStream().close(); } catch (IOException ignored) { }
            boolean reaped = false;
            try { reaped = child.waitFor(8, TimeUnit.SECONDS); }
            catch (InterruptedException stopped) { interrupted = true; }
            if (!reaped) {
                child.destroyForcibly();
                try { reaped = child.waitFor(2, TimeUnit.SECONDS); }
                catch (InterruptedException stopped) { interrupted = true; }
            }
            try { child.getInputStream().close(); } catch (IOException ignored) { }
            if (!reaped) throw new NodeFailure(NodeFailure.Code.CLEANUP);
        } finally { if (interrupted) Thread.currentThread().interrupt(); }
    }
}
