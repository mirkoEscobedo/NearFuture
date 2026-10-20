package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import java.nio.file.Path;
import java.util.HexFormat;
import org.nearfuture.protocol.v1.AccountId;
import org.nearfuture.protocol.v1.DeviceId;
import org.nearfuture.protocol.v1.Principal;

/** Explicit application-startup boundary. No configuration lookup is needed in a game callback. */
public final class DiagnosticConfiguration {
    private static final String PREFIX = "nf.ipc.diagnostic.";
    private DiagnosticConfiguration() { }
    public static NodeLaunch configured() {
        if (!"true".equals(System.getProperty(PREFIX + "enabled"))) return null;
        try {
            Principal principal = Principal.newBuilder()
                .setAccountId(AccountId.newBuilder().setValue(hex("account", 16)))
                .setDeviceId(DeviceId.newBuilder().setValue(hex("device", 16))).build();
            var scope = new ParentHandoff.Scope(hex("universe", 16), hex("history", 16),
                hex("ruleset", 32), hex("content", 32), principal);
            return new NodeLaunch(Path.of(value("executable")), hex("sha256", 32),
                Path.of(value("vault")), Path.of(value("saves")), value("publication"), scope, 60000);
        } catch (RuntimeException invalid) { throw new NodeFailure(NodeFailure.Code.INVALID); }
    }
    private static String value(String name) {
        String value = System.getProperty(PREFIX + name);
        if (value == null || value.isEmpty() || value.length() > 4096) throw new NodeFailure(NodeFailure.Code.INVALID);
        return value;
    }
    private static ByteString hex(String name, int bytes) {
        String value = value(name);
        if (value.length() != bytes * 2 || !value.matches("[0-9a-f]+")) throw new NodeFailure(NodeFailure.Code.INVALID);
        return ByteString.copyFrom(HexFormat.of().parseHex(value));
    }
}
