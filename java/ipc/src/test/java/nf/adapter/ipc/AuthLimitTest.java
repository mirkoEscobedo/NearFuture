package nf.adapter.ipc;
import com.google.protobuf.ByteString;
import java.util.Arrays;
import org.nearfuture.ipc.v1.*;
final class AuthLimitTest {
    private AuthLimitTest() { }
    static void run() {
        byte[] key=new byte[32];new java.security.SecureRandom().nextBytes(key);
        var limits=AuthClientTest.limits();var lower=limits.toBuilder().setInflightItems(16).setChunkBytes(4096).setTransferBytes(524288).build();
        var context=AuthClientTest.context(1,12345,9);
        try(var client=new MutualAuthClient(context,key,lower)) {
            byte[] nonce=client.hello().getHello().getClientNonce().toByteArray(),server=new byte[32];
            var challenge=AuthClientTest.header(9).setChallenge(LocalAuthChallenge.newBuilder().setServerNonce(ByteString.copyFrom(server)).setSelectedLimits(lower).setServerProof(ByteString.copyFrom(AuthTranscript.proof(key,AuthTranscript.encode(context,lower,lower,nonce,server,1))))).build();
            client.respond(challenge.toByteArray());
            var accepted=AuthClientTest.header(9).setAccepted(LocalAuthAccepted.newBuilder().setServerFinishedProof(ByteString.copyFrom(AuthTranscript.proof(key,AuthTranscript.encode(context,lower,lower,nonce,server,3))))).build();
            client.finish(accepted.toByteArray());AuthClientTest.check(client.session().limits().equals(lower));
            AuthClientTest.expectFailure(()->client.finish(accepted.toByteArray()));AuthClientTest.check(!client.isActive());
        }
        AuthClientTest.expectFailure(()->new MutualAuthClient(context,key,limits.toBuilder().setDecodedBytes(100).build()));
        AuthClientTest.expectFailure(()->new MutualAuthClient(context,key,limits.toBuilder().setInflightItems(-1).build()));
        AuthClientTest.expectFailure(()->AuthClientTest.context(0,12345,9));AuthClientTest.expectFailure(()->AuthClientTest.context(1,0,9));AuthClientTest.expectFailure(()->AuthClientTest.context(1,12345,0));
        Arrays.fill(key,(byte)0);
    }
}