package nf.adapter.ipc;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.Arrays;
import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Malformed private input never mints an authenticated client or prints its bytes. */
public final class ParentHandoffRejectionTest {
    private ParentHandoffRejectionTest() { }
    private static void rejected(byte[] packet, byte[] nonce, ParentHandoff.Scope scope) {
        try (ParentHandoff unexpected = ParentHandoff.decode(packet, nonce, scope)) {
            unexpected.context(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL);
            throw new AssertionError("malformed private handoff admitted");
        } catch (AuthFailure failure) {
            if (failure.code() != AuthFailure.Code.INVALID || !failure.getMessage().equals("INVALID")) {
                throw new AssertionError("private refusal must be typed and redacted");
            }
        } finally {
            if (packet != null) Arrays.fill(packet, (byte)0);
        }
    }
    public static void main(String[] args) {
        for (int length : new int[]{0, 262, 264}) {
            rejected(Arrays.copyOf(ParentHandoffFixture.packet(), length), ParentHandoffFixture.nonce(), ParentHandoffFixture.scope());
        }
        rejected(null, ParentHandoffFixture.nonce(), ParentHandoffFixture.scope());
        rejected(ParentHandoffFixture.packet(), null, ParentHandoffFixture.scope());
        rejected(ParentHandoffFixture.packet(), new byte[32], ParentHandoffFixture.scope());
        rejected(ParentHandoffFixture.packet(), ParentHandoffFixture.repeated(32, 0x72), ParentHandoffFixture.scope());
        rejected(ParentHandoffFixture.packet(), ParentHandoffFixture.nonce(), null);
        // Fixed offsets come from the literal format, not the production decoder.
        for (int offset : new int[]{0, 13, 45, 67, 83, 99, 131, 163, 179}) {
            byte[] packet = ParentHandoffFixture.packet(); packet[offset] ^= 1;
            rejected(packet, ParentHandoffFixture.nonce(), ParentHandoffFixture.scope());
        }
        byte[] duplicate = ParentHandoffFixture.packet();
        duplicate[57] = duplicate[55]; duplicate[58] = duplicate[56];
        rejected(duplicate, ParentHandoffFixture.nonce(), ParentHandoffFixture.scope());
        for (int[] range : new int[][]{{55,57}, {59,67}, {195,227}}) {
            byte[] packet = ParentHandoffFixture.packet(); Arrays.fill(packet, range[0], range[1], (byte)0);
            rejected(packet, ParentHandoffFixture.nonce(), ParentHandoffFixture.scope());
        }
        for (int offset : new int[]{227,231,235,239,243,247,251}) {
            byte[] packet = ParentHandoffFixture.packet();
            ByteBuffer.wrap(packet).order(ByteOrder.LITTLE_ENDIAN).putInt(offset, 0);
            rejected(packet, ParentHandoffFixture.nonce(), ParentHandoffFixture.scope());
        }
        byte[] signedTransfer = ParentHandoffFixture.packet();
        ByteBuffer.wrap(signedTransfer).order(ByteOrder.LITTLE_ENDIAN).putLong(255, -1);
        rejected(signedTransfer, ParentHandoffFixture.nonce(), ParentHandoffFixture.scope());
        byte[] packet = ParentHandoffFixture.packet();
        ParentHandoff closed = ParentHandoff.decode(packet, ParentHandoffFixture.nonce(), ParentHandoffFixture.scope());
        Arrays.fill(packet, (byte)0); closed.close();
        try {
            closed.client(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL);
            throw new AssertionError("closed private owner minted a client");
        } catch (AuthFailure failure) {
            if (failure.code() != AuthFailure.Code.STATE) throw new AssertionError("closed owner state");
        }
        System.out.println("Private handoff length, nonce, context, endpoint, limits and closed-owner refusals passed.");
    }
}
