package nf.probe;

import java.io.IOException;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.channels.SocketChannel;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.Arrays;
import java.util.function.BooleanSupplier;
import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;

/** Worker-only peer of docs/modules/probe-v1.md. Credentials/endpoint come directly from
 * an owned bootstrap, never ambient files, command arguments or network discovery. */
public final class ModuleProbeClient implements ProbeTransport {
    private final SocketChannel socket;
    private final byte[] session;
    private byte[] key;
    private long sequence;
    private boolean closed;

    private ModuleProbeClient(int port, byte[] session) throws IOException {
        if (port < 1 || port > 65535 || session == null || session.length != 16 || zero(session))
            throw new IOException("Invalid probe bootstrap");
        this.session = session.clone();
        socket = SocketChannel.open();
        try {
            socket.configureBlocking(false);
            socket.connect(new InetSocketAddress(InetAddress.getByAddress(new byte[]{127, 0, 0, 1}), port));
        } catch (IOException | RuntimeException failure) { socket.close(); throw failure; }
    }

    /** Exactly one authenticated negotiation, with the caller's finite deadline and cancellation. */
    public static ModuleProbeClient connect(int port, byte[] session, byte[] token,
            ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        if (token == null || token.length != 32 || zero(token) || request == null || cancelled == null)
            throw new IOException("Invalid probe bootstrap");
        ModuleProbeClient client = new ModuleProbeClient(port, session);
        try { client.negotiate(port, token, request.boundedTo(2000), cancelled); return client; }
        catch (IOException | RuntimeException | LinkageError failure) { client.close(); throw failure; }
    }

    private void negotiate(int port, byte[] token, ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        byte[] secret = token.clone();
        byte[] nonce = new byte[16];
        byte[] hello = null;
        try {
            new SecureRandom().nextBytes(nonce);
            if (zero(nonce)) throw new IOException("Random unavailable");
            ByteBuffer h = ByteBuffer.allocate(80);
            magic(h, "NFMHEL01"); h.put(session).putShort((short)port).putShort((short)1).putShort((short)1)
                    .putShort((short)0).put(nonce).putInt(1).putInt(1).putInt(136).putInt(122).putInt(1000);
            hello = signed(secret, "NF-MODULE-HELLO-1", h.array());
            while (!socket.isConnected() && !socket.finishConnect()) pause(request, cancelled);
            send(hello, request, cancelled);
            byte[] accepted = receive(120, request, cancelled);
            if (accepted.length != 120) throw new IOException("Invalid probe negotiation");
            byte[] data = Arrays.copyOf(accepted, 88);
            verify(secret, "NF-MODULE-ACCEPT-1", accepted, 88, hello, data);
            ByteBuffer a = ByteBuffer.wrap(data);
            requireMagic(a, "NFMACK01"); requireBytes(a, session);
            int version = Short.toUnsignedInt(a.getShort()), status = Short.toUnsignedInt(a.getShort());
            int available = a.getInt(), granted = a.getInt();
            int requestBytes = a.getInt(), responseBytes = a.getInt(), timeout = a.getInt();
            byte[] serverNonce = new byte[16]; a.get(serverNonce);
            if (version != 1 || status != 0 || (available & 1) == 0 || granted != 1
                    || requestBytes != 136 || responseBytes != 122 || timeout != 1000 || zero(serverNonce))
                throw new IOException("Probe negotiation rejected");
            while (a.hasRemaining()) if (a.get() != 0) throw new IOException("Invalid probe negotiation");
            key = mac(secret, "NF-MODULE-KEY-1", hello, data);
        } finally {
            Arrays.fill(secret, (byte)0); Arrays.fill(nonce, (byte)0);
            if (hello != null) Arrays.fill(hello, (byte)0);
        }
    }

    @Override public ProbeResult exchange(ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        check(request, cancelled);
        if (closed || key == null || sequence == Long.MAX_VALUE) throw new IOException("Probe session closed");
        long next = ++sequence;
        long remaining = (request.deadlineNanos() - System.nanoTime()) / 1_000_000L;
        int timeout = (int)Math.min(1000, remaining);
        if (timeout < 1) throw new IOException("Probe deadline");
        ProbeRequest application = request.boundedTo(timeout);
        ByteBuffer data = ByteBuffer.allocate(104);
        magic(data, "NFMREQ01"); data.put(session).put(request.id()).putLong(request.generation()).putLong(next)
                .putShort((short)1).putShort((short)1).putShort((short)1).putShort((short)0)
                .putInt(1).putInt(timeout).put(request.challenge());
        send(signed(key, "NF-MODULE-REQUEST-1", data.array()), application, cancelled);
        byte[] reply = receive(122, application, cancelled);
        if (reply.length < 90) throw new IOException("Invalid probe reply");
        int end = reply.length - 32;
        verify(key, "NF-MODULE-REPLY-1", reply, end, Arrays.copyOf(reply, end));
        ByteBuffer r = ByteBuffer.wrap(reply, 0, end);
        requireMagic(r, "NFMRSP01"); requireBytes(r, session); requireBytes(r, request.id());
        if (r.getLong() != request.generation() || r.getLong() != next) throw new IOException("Stale probe reply");
        int status = Short.toUnsignedInt(r.getShort());
        if (status > 12 || status != 0 && r.hasRemaining()) throw new IOException("Invalid probe reply");
        byte[] payload = new byte[r.remaining()]; r.get(payload);
        if (status == 0 && !MessageDigest.isEqual(payload, request.challenge())) throw new IOException("Invalid probe echo");
        check(application, cancelled);
        ProbeResult.Status outcome = switch (status) {
            case 0 -> ProbeResult.Status.REPLIED;
            case 9 -> ProbeResult.Status.DEADLINE;
            case 10 -> ProbeResult.Status.CANCELLED;
            case 11 -> ProbeResult.Status.UNAVAILABLE;
            default -> ProbeResult.Status.REJECTED;
        };
        return new ProbeResult(request.generation(), request.id(), session, outcome, payload);
    }

    private void send(byte[] body, ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        ByteBuffer frame = ByteBuffer.allocate(body.length + 4).putInt(body.length).put(body); frame.flip();
        while (frame.hasRemaining()) {
            check(request, cancelled);
            int limit = frame.limit(); frame.limit(Math.min(limit, frame.position() + 4096));
            try { if (socket.write(frame) == 0) pause(request, cancelled); }
            finally { frame.limit(limit); }
        }
    }
    private byte[] receive(int maximum, ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        ByteBuffer header = ByteBuffer.allocate(4); read(header, request, cancelled); header.flip();
        long length = Integer.toUnsignedLong(header.getInt());
        if (length == 0 || length > maximum) throw new IOException("Invalid bounded probe frame");
        ByteBuffer body = ByteBuffer.allocate((int)length); read(body, request, cancelled); return body.array();
    }
    private void read(ByteBuffer target, ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        while (target.hasRemaining()) {
            check(request, cancelled);
            int count = socket.read(target);
            if (count < 0) throw new IOException("Probe disconnected");
            if (count == 0) pause(request, cancelled);
        }
    }
    private static void pause(ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        check(request, cancelled);
        try { Thread.sleep(1); }
        catch (InterruptedException interrupted) { Thread.currentThread().interrupt(); throw new IOException("Probe interrupted"); }
    }
    private static void check(ProbeRequest request, BooleanSupplier cancelled) throws IOException {
        if (request.expired() || cancelled.getAsBoolean()) throw new IOException("Probe cancelled or expired");
    }
    private static byte[] mac(byte[] key, String domain, byte[]... parts) throws IOException {
        try {
            Mac hmac = Mac.getInstance("HmacSHA256"); hmac.init(new SecretKeySpec(key, "HmacSHA256"));
            hmac.update(domain.getBytes(StandardCharsets.US_ASCII)); hmac.update((byte)0);
            for (byte[] part : parts) hmac.update(part);
            return hmac.doFinal();
        } catch (GeneralSecurityException unavailable) { throw new IOException("Probe authentication unavailable"); }
    }
    private static byte[] signed(byte[] key, String domain, byte[] data) throws IOException {
        return ByteBuffer.allocate(data.length + 32).put(data).put(mac(key, domain, data)).array();
    }
    private static void verify(byte[] key, String domain, byte[] signed, int end, byte[]... parts) throws IOException {
        if (!MessageDigest.isEqual(mac(key, domain, parts), Arrays.copyOfRange(signed, end, signed.length)))
            throw new IOException("Probe authentication rejected");
    }
    private static void magic(ByteBuffer buffer, String value) { buffer.put(value.getBytes(StandardCharsets.US_ASCII)); }
    private static void requireMagic(ByteBuffer buffer, String value) throws IOException {
        requireBytes(buffer, value.getBytes(StandardCharsets.US_ASCII));
    }
    private static void requireBytes(ByteBuffer buffer, byte[] expected) throws IOException {
        byte[] actual = new byte[expected.length]; buffer.get(actual);
        if (!MessageDigest.isEqual(actual, expected)) throw new IOException("Invalid probe binding");
    }
    private static boolean zero(byte[] value) { for (byte b : value) if (b != 0) return false; return true; }
    @Override public void close() throws IOException {
        closed = true;
        if (key != null) Arrays.fill(key, (byte)0);
        socket.close();
    }
}
