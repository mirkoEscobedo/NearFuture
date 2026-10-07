package nf.adapter.ipc;
import com.google.protobuf.ByteString;
import java.util.Arrays;
import org.nearfuture.ipc.v1.*;
import org.nearfuture.protocol.v1.ResourceLimits;
final class AuthRejectionTest {
    private AuthRejectionTest() { }
    static void run() {
        byte[] key=new byte[32];new java.security.SecureRandom().nextBytes(key);
        var context=AuthClientTest.context(1,12345,9);
        for(int mutation=0;mutation<13;mutation++) {
            try(var client=new MutualAuthClient(context,key,AuthClientTest.limits())) {
                byte[] nonce=client.hello().getHello().getClientNonce().toByteArray(),server=new byte[32];Arrays.fill(server,(byte)9);
                AuthContext bound=context;byte[] secret=key.clone();int stage=1;ResourceLimits selected=AuthClientTest.limits();long session=9;
                switch(mutation) {
                    case 0 -> secret[0]^=1;
                    case 1 -> stage=2;
                    case 2 -> bound=AuthClientTest.context(2,12345,9);
                    case 3 -> bound=AuthClientTest.context(1,12346,9);
                    case 4 -> bound=AuthClientTest.context(1,12345,10);
                    case 5 -> nonce[0]^=1;
                    case 6 -> bound=change(context,AuthClientTest.repeated(7,16),context.history(),context.ruleset(),context.content(),context.principal());
                    case 7 -> bound=change(context,context.universe(),AuthClientTest.repeated(7,16),context.ruleset(),context.content(),context.principal());
                    case 8 -> bound=change(context,context.universe(),context.history(),AuthClientTest.repeated(7,32),context.content(),context.principal());
                    case 9 -> bound=change(context,context.universe(),context.history(),context.ruleset(),AuthClientTest.repeated(7,32),context.principal());
                    case 10 -> bound=change(context,context.universe(),context.history(),context.ruleset(),context.content(),context.principal().toBuilder().setDeviceId(context.principal().getDeviceId().toBuilder().setValue(AuthClientTest.repeated(7,16))).build());
                    case 11 -> selected=selected.toBuilder().setInflightItems(31).build();
                    case 12 -> session=10;
                    default -> throw new AssertionError();
                }
                byte[] proof=AuthTranscript.proof(secret,AuthTranscript.encode(bound,AuthClientTest.limits(),AuthClientTest.limits(),nonce,server,stage));
                var challenge=AuthClientTest.header(session).setChallenge(LocalAuthChallenge.newBuilder().setServerNonce(ByteString.copyFrom(server)).setSelectedLimits(selected).setServerProof(ByteString.copyFrom(proof))).build();
                AuthClientTest.expectFailure(()->client.respond(challenge.toByteArray()));AuthClientTest.check(!client.isActive());AuthClientTest.expectFailure(client::hello);
            }
        }
        try(var client=new MutualAuthClient(context,key,AuthClientTest.limits())) {
            byte[] nonce=client.hello().getHello().getClientNonce().toByteArray(),server=new byte[32];
            var challenge=AuthClientTest.header(9).setChallenge(LocalAuthChallenge.newBuilder().setServerNonce(ByteString.copyFrom(server)).setSelectedLimits(AuthClientTest.limits()).setServerProof(ByteString.copyFrom(AuthTranscript.proof(key,AuthTranscript.encode(context,AuthClientTest.limits(),AuthClientTest.limits(),nonce,server,1))))).build();
            client.respond(challenge.toByteArray());
            var reflection=AuthClientTest.header(9).setAccepted(LocalAuthAccepted.newBuilder().setServerFinishedProof(challenge.getChallenge().getServerProof())).build();
            AuthClientTest.expectFailure(()->client.finish(reflection.toByteArray()));AuthClientTest.check(!client.isActive());
        }
        try(var client=new MutualAuthClient(context,key,AuthClientTest.limits())) {AuthClientTest.expectFailure(()->client.finish(new byte[0]));AuthClientTest.check(!client.isActive());}
        Arrays.fill(key,(byte)0);
    }
    private static AuthContext change(AuthContext c,ByteString universe,ByteString history,ByteString ruleset,ByteString content,org.nearfuture.protocol.v1.Principal principal) {return new AuthContext(c.role(),c.port(),c.runtimeSession(),universe,history,ruleset,content,principal,c.localLimits());}
}