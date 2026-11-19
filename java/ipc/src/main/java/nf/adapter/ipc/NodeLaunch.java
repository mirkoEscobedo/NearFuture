package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import java.nio.file.Path;

/** Trusted background-owner inputs. Paths are private and must not become diagnostics. */
public record NodeLaunch(Path executable, ByteString executableSha256, Path vault, Path saves,
                         String publication, ParentHandoff.Scope scope, long durationMillis) {
    public NodeLaunch {
        if (executable == null || !executable.isAbsolute() || executableSha256 == null
            || executableSha256.size() != 32 || vault == null || !vault.isAbsolute()
            || saves == null || !saves.isAbsolute() || publication == null
            || !publication.matches("[A-Za-z0-9_-]{1,64}") || scope == null || !validScope(scope)
            || durationMillis < 100 || durationMillis > 60000) {
            throw new NodeFailure(NodeFailure.Code.INVALID);
        }
    }
    private static boolean validScope(ParentHandoff.Scope scope) {
        var principal = scope.principal();
        return scope.universe().size() == 16 && scope.history().size() == 16
            && scope.ruleset().size() == 32 && scope.content().size() == 32
            && principal.hasAccountId() && principal.hasDeviceId()
            && principal.getUnknownFields().asMap().isEmpty()
            && principal.getAccountId().getUnknownFields().asMap().isEmpty()
            && principal.getDeviceId().getUnknownFields().asMap().isEmpty()
            && principal.getAccountId().getValue().size() == 16
            && principal.getDeviceId().getValue().size() == 16;
    }
    @Override public String toString() { return "PrivateNodeLaunch"; }
}
