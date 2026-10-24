package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import org.nearfuture.protocol.v1.AccountId;
import org.nearfuture.protocol.v1.DeviceId;
import org.nearfuture.protocol.v1.Principal;

/** Independent fixed wire oracle shared by admission and refusal controls. */
final class ParentHandoffFixture {
    private ParentHandoffFixture() { }
    static byte[] repeated(int count, int value) {
        byte[] bytes = new byte[count]; Arrays.fill(bytes, (byte)value); return bytes;
    }
    static byte[] nonce() { return repeated(32, 0x71); }
    static Principal principal() {
        return Principal.newBuilder()
            .setAccountId(AccountId.newBuilder().setValue(ByteString.copyFrom(repeated(16, 5))))
            .setDeviceId(DeviceId.newBuilder().setValue(ByteString.copyFrom(repeated(16, 6))))
            .build();
    }
    static ParentHandoff.Scope scope() {
        return new ParentHandoff.Scope(ByteString.copyFrom(repeated(16, 1)),
            ByteString.copyFrom(repeated(16, 2)), ByteString.copyFrom(repeated(32, 3)),
            ByteString.copyFrom(repeated(32, 4)), principal());
    }
    static byte[] packet() {
        ByteBuffer packet = ByteBuffer.allocate(263).order(ByteOrder.LITTLE_ENDIAN);
        packet.put("NF-HANDOFF-1\0".getBytes(StandardCharsets.US_ASCII)).put(nonce())
            .put("NF-IPC-R2\0".getBytes(StandardCharsets.US_ASCII))
            .putShort((short)19601).putShort((short)19602).putLong(9)
            .put(repeated(16, 1)).put(repeated(16, 2)).put(repeated(32, 3)).put(repeated(32, 4))
            .put(repeated(16, 5)).put(repeated(16, 6)).put(repeated(32, 0x7a))
            .putInt(16384).putInt(8192).putInt(262144).putInt(32)
            .putInt(16384).putInt(256).putInt(16).putLong(1048576);
        if (packet.position() != 263) throw new AssertionError("independent packet width");
        return packet.array();
    }
}
