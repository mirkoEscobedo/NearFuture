package nf.adapter.ipc;

import java.util.Arrays;
import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Independent literal private-handoff format oracle; no production encoder is used. */
public final class ParentHandoffTest {
    private ParentHandoffTest() { }
    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
    public static void main(String[] args) {
        byte[] nonce = ParentHandoffFixture.nonce();
        byte[] packet = ParentHandoffFixture.packet();
        ParentHandoff.Scope scope = ParentHandoffFixture.scope();
        try (ParentHandoff handoff = ParentHandoff.decode(packet, nonce, scope)) {
            AuthContext control = handoff.context(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL);
            AuthContext bulk = handoff.context(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK);
            check(control.port() == 19601 && bulk.port() == 19602, "distinct exact endpoints");
            check(control.runtimeSession() == 9 && bulk.runtimeSession() == 9, "exact runtime session");
            check(control.universe().equals(scope.universe()) && control.history().equals(scope.history()), "scope binding");
            check(control.ruleset().equals(scope.ruleset()) && control.content().equals(scope.content()), "policy binding");
            check(control.principal().equals(ParentHandoffFixture.principal())
                && bulk.principal().equals(ParentHandoffFixture.principal()), "principal binding");
            check(control.localLimits().getControlFrameBytes() == 16384
                && bulk.localLimits().getTransferBytes() == 1048576, "bounded advertised limits");
        } finally {
            Arrays.fill(packet, (byte)0);
        }
        System.out.println("Private parent descriptor admitted with exact expected scope; no native lifecycle claim.");
    }
}
