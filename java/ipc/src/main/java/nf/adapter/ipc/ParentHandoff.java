package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Arrays;
import org.nearfuture.ipc.v1.LocalEndpointRole;
import org.nearfuture.protocol.v1.AccountId;
import org.nearfuture.protocol.v1.DeviceId;
import org.nearfuture.protocol.v1.Principal;
import org.nearfuture.protocol.v1.ResourceLimits;

/** Bounded private parent-pipe input. Never pass this owner or its raw input to a game callback. */
public final class ParentHandoff implements AutoCloseable {
    public static final int BYTES = 263;

    public record Scope(ByteString universe, ByteString history, ByteString ruleset,
                        ByteString content, Principal principal) {
        public Scope {
            if (universe == null || history == null || ruleset == null || content == null || principal == null) AuthLimits.fail();
        }
        private boolean matches(AuthContext context) {
            return universe.equals(context.universe()) && history.equals(context.history())
                && ruleset.equals(context.ruleset()) && content.equals(context.content())
                && principal.equals(context.principal());
        }
    }

    private final AuthContext control;
    private final AuthContext bulk;
    private final byte[] token;
    private boolean closed;

    private ParentHandoff(AuthContext control, AuthContext bulk, byte[] token) {
        this.control = control;
        this.bulk = bulk;
        this.token = token;
    }

    /** Caller exclusively owns input and must erase its raw secret bytes after this call. */
    public static ParentHandoff decode(byte[] input, byte[] parentNonce, Scope expected) {
        if (input == null || input.length != BYTES || parentNonce == null || parentNonce.length != 32
            || allZero(parentNonce) || expected == null) AuthLimits.fail();
        ByteBuffer buffer = ByteBuffer.wrap(input).order(ByteOrder.LITTLE_ENDIAN);
        magic(buffer, "NF-HANDOFF-1\0");
        byte[] echoed = new byte[32]; buffer.get(echoed);
        if (!MessageDigest.isEqual(echoed, parentNonce)) AuthLimits.fail();
        magic(buffer, "NF-IPC-R2\0");
        int controlPort = Short.toUnsignedInt(buffer.getShort());
        int bulkPort = Short.toUnsignedInt(buffer.getShort());
        if (controlPort == bulkPort) AuthLimits.fail();
        long session = buffer.getLong();
        ByteString universe = bytes(buffer, 16), history = bytes(buffer, 16);
        ByteString ruleset = bytes(buffer, 32), content = bytes(buffer, 32);
        Principal principal = Principal.newBuilder()
            .setAccountId(AccountId.newBuilder().setValue(bytes(buffer, 16)))
            .setDeviceId(DeviceId.newBuilder().setValue(bytes(buffer, 16))).build();
        byte[] secret = new byte[32]; buffer.get(secret);
        boolean transferred = false;
        try {
            if (allZero(secret)) AuthLimits.fail();
            ResourceLimits limits = ResourceLimits.newBuilder()
                .setControlFrameBytes(buffer.getInt()).setChunkBytes(buffer.getInt())
                .setInflightBytes(buffer.getInt()).setInflightItems(buffer.getInt())
                .setDecodedBytes(buffer.getInt()).setCollectionItems(buffer.getInt())
                .setNestingDepth(buffer.getInt()).setTransferBytes(buffer.getLong()).build();
            AuthContext control = new AuthContext(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL,
                controlPort, session, universe, history, ruleset, content, principal, limits);
            AuthContext bulk = new AuthContext(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK,
                bulkPort, session, universe, history, ruleset, content, principal, limits);
            if (buffer.hasRemaining() || !expected.matches(control)) AuthLimits.fail();
            ParentHandoff handoff = new ParentHandoff(control, bulk, secret);
            transferred = true;
            return handoff;
        } finally {
            if (!transferred) Arrays.fill(secret, (byte)0);
        }
    }

    public AuthContext context(LocalEndpointRole role) {
        if (closed) throw new AuthFailure(AuthFailure.Code.STATE);
        if (role == LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL) return control;
        if (role == LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK) return bulk;
        throw new AuthFailure(AuthFailure.Code.INVALID);
    }

    public MutualAuthClient client(LocalEndpointRole role) {
        AuthContext context = context(role);
        return new MutualAuthClient(context, token, context.localLimits());
    }

    private static ByteString bytes(ByteBuffer buffer, int count) {
        byte[] bytes = new byte[count]; buffer.get(bytes); return ByteString.copyFrom(bytes);
    }
    private static void magic(ByteBuffer buffer, String expected) {
        byte[] bytes = new byte[expected.length()]; buffer.get(bytes);
        if (!Arrays.equals(bytes, expected.getBytes(StandardCharsets.US_ASCII))) AuthLimits.fail();
    }
    private static boolean allZero(byte[] bytes) {
        int bits = 0; for (byte value : bytes) bits |= value; return bits == 0;
    }
    @Override public void close() { Arrays.fill(token, (byte)0); closed = true; }
}
