package nf.probe;

import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;

/** Independent malicious peer fixtures exercise Java's receive trust boundary. */
public final class ProbeWireTest {
    private ProbeWireTest() { }
    private enum Fault { NONE, TAG, SESSION, VERSION, LIMIT, NONCE, RESERVED, REPLY_ID,
        REPLY_GENERATION, REPLY_SEQUENCE, REPLY_TAG, OVERSIZE, LATE }
    private static final byte[] TOKEN = filled(32, 4), SESSION = filled(16, 7);

    public static void main(String[] args) throws Exception {
        for (Fault fault : Fault.values()) scenario(fault);
        System.out.println("Probe wire: 13 fragmented/hostile peer cases PASS.");
    }
    private static void scenario(Fault fault) throws Exception {
        AtomicReference<Throwable> failure = new AtomicReference<>();
        try (ServerSocket listener = new ServerSocket(0, 1, InetAddress.getByAddress(new byte[]{127, 0, 0, 1}))) {
            listener.setSoTimeout(3000);
            Thread peer = new Thread(() -> {
                try (Socket socket = listener.accept()) {
                    socket.setSoTimeout(3000);
                    DataInputStream input = new DataInputStream(socket.getInputStream());
                    DataOutputStream output = new DataOutputStream(socket.getOutputStream());
                    byte[] hello = frame(input, 112);
                    ByteBuffer accept = ByteBuffer.allocate(88);
                    accept.put(ascii("NFMACK01")).put(SESSION).putShort((short)1).putShort((short)0)
                            .putInt(1).putInt(1).putInt(136).putInt(122).putInt(1000).put(filled(16, 8));
                    if (fault == Fault.SESSION) accept.put(8, (byte)99);
                    if (fault == Fault.VERSION) accept.putShort(24, (short)2);
                    if (fault == Fault.LIMIT) accept.putInt(40, 346);
                    if (fault == Fault.NONCE) Arrays.fill(accept.array(), 48, 64, (byte)0);
                    if (fault == Fault.RESERVED) accept.put(87, (byte)1);
                    byte[] tag = hmac(TOKEN, "NF-MODULE-ACCEPT-1", hello, accept.array());
                    if (fault == Fault.TAG) tag[0] ^= 1;
                    sendFragmented(output, concat(accept.array(), tag));
                    if (fault.ordinal() >= Fault.REPLY_ID.ordinal() || fault == Fault.NONE) {
                        byte[] request = frame(input, 136);
                        if (fault == Fault.OVERSIZE) { output.writeInt(0x7fffffff); output.flush(); return; }
                        byte[] key = hmac(TOKEN, "NF-MODULE-KEY-1", hello, accept.array());
                        ByteBuffer reply = ByteBuffer.allocate(90);
                        reply.put(ascii("NFMRSP01")).put(SESSION).put(request, 24, 32)
                                .putShort((short)0).put(request, 72, 32);
                        if (fault == Fault.REPLY_ID) reply.put(24, (byte)99);
                        if (fault == Fault.REPLY_GENERATION) reply.putLong(40, 99);
                        if (fault == Fault.REPLY_SEQUENCE) reply.putLong(48, 99);
                        byte[] proof = hmac(key, "NF-MODULE-REPLY-1", reply.array());
                        if (fault == Fault.REPLY_TAG) proof[0] ^= 1;
                        if (fault == Fault.LATE) Thread.sleep(1200);
                        sendFragmented(output, concat(reply.array(), proof));
                    }
                } catch (IOException expectedOnRefusal) {
                    if (fault == Fault.NONE) failure.set(expectedOnRefusal);
                } catch (Throwable problem) { failure.set(problem); }
            }, "nf-probe-hostile-fixture");
            peer.setDaemon(true); peer.start();
            ProbeRequest request = new ProbeRequest(1, filled(16, 1), filled(32, 2), System.nanoTime() + TimeUnit.SECONDS.toNanos(4));
            boolean accepted = false;
            try (ModuleProbeClient client = ModuleProbeClient.connect(listener.getLocalPort(), SESSION, TOKEN, request, () -> false)) {
                accepted = client.exchange(request, () -> false).status() == ProbeResult.Status.REPLIED;
            } catch (IOException refused) { if (fault == Fault.NONE) throw refused; }
            peer.join(4000);
            if (peer.isAlive()) throw new AssertionError("Peer fixture did not stop");
            if (failure.get() != null) throw new AssertionError(fault.name(), failure.get());
            if (accepted != (fault == Fault.NONE)) throw new AssertionError("Trust boundary: " + fault);
        }
    }
    private static byte[] frame(DataInputStream input, int exact) throws IOException {
        if (input.readInt() != exact) throw new IOException("Fixture length");
        byte[] bytes = input.readNBytes(exact);
        if (bytes.length != exact) throw new IOException("Fixture EOF");
        return bytes;
    }
    private static void sendFragmented(DataOutputStream output, byte[] body) throws IOException {
        byte[] frame = ByteBuffer.allocate(body.length + 4).putInt(body.length).put(body).array();
        for (int offset = 0; offset < frame.length; offset += 3) {
            output.write(frame, offset, Math.min(3, frame.length - offset)); output.flush();
        }
    }
    private static byte[] hmac(byte[] key, String label, byte[]... parts) throws Exception {
        Mac mac = Mac.getInstance("HmacSHA256"); mac.init(new SecretKeySpec(key, "HmacSHA256"));
        mac.update(ascii(label)); mac.update((byte)0);
        for (byte[] bytes : parts) mac.update(bytes);
        return mac.doFinal();
    }
    private static byte[] filled(int length, int value) { byte[] bytes = new byte[length]; Arrays.fill(bytes, (byte)value); return bytes; }
    private static byte[] ascii(String value) { return value.getBytes(StandardCharsets.US_ASCII); }
    private static byte[] concat(byte[] first, byte[] second) { return ByteBuffer.allocate(first.length + second.length).put(first).put(second).array(); }
}
