package nf.adapter.ipc;
import com.google.protobuf.ByteString;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.regex.Pattern;
import org.nearfuture.ipc.v1.*;
import org.nearfuture.protocol.v1.*;
public final class AuthClientTest {
    private AuthClientTest() { }
    public static void main(String[] arguments) throws Exception {
        String fixture=Files.readString(Path.of("protocol/vectors/local-auth-v1.json"));
        byte[] key=hex(value(fixture,"keyHex"));
        AuthContext context=context(1,12345,9);
        byte[] clientNonce=hex(value(fixture,"clientNonce")),serverNonce=hex(value(fixture,"serverNonce"));
        var vectors=Pattern.compile("\"stage\": (\\d+),\\s*\"transcriptHex\": \"([0-9a-f]+)\",\\s*\"proofHex\": \"([0-9a-f]+)\"").matcher(fixture);
        int count=0;while(vectors.find()) {
            int stage=Integer.parseInt(vectors.group(1));
            byte[] transcript=AuthTranscript.encode(context,limits(),limits(),clientNonce,serverNonce,stage);
            check(Arrays.equals(transcript,hex(vectors.group(2))));
            check(Arrays.equals(AuthTranscript.proof(key,transcript),hex(vectors.group(3))));count++;
        } check(count==3);
        clientRoundTrip(context,key);AuthRejectionTest.run();AuthLimitTest.run();
        System.out.println("PASS: Java mutual auth transcripts, ordered activation and rejection");
    }
    static void clientRoundTrip(AuthContext context,byte[] key) {
        try(var client=new MutualAuthClient(context,key,limits());var other=new MutualAuthClient(context,key,limits())) {
            var hello=client.hello();check(!hello.getHello().getClientNonce().equals(other.hello().getHello().getClientNonce()));
            byte[] nonce=hello.getHello().getClientNonce().toByteArray(),server=new byte[32];Arrays.fill(server,(byte)9);
            byte[] proof=AuthTranscript.proof(key,AuthTranscript.encode(context,limits(),limits(),nonce,server,1));
            var challenge=header(9).setChallenge(LocalAuthChallenge.newBuilder().setServerNonce(ByteString.copyFrom(server)).setSelectedLimits(limits()).setServerProof(ByteString.copyFrom(proof))).build();
            var response=client.respond(challenge.toByteArray());check(!client.isActive());
            check(Arrays.equals(response.getProof().getClientProof().toByteArray(),AuthTranscript.proof(key,AuthTranscript.encode(context,limits(),limits(),nonce,server,2))));
            byte[] finished=AuthTranscript.proof(key,AuthTranscript.encode(context,limits(),limits(),nonce,server,3));
            client.finish(header(9).setAccepted(LocalAuthAccepted.newBuilder().setServerFinishedProof(ByteString.copyFrom(finished))).build().toByteArray());check(client.isActive());
            check(client.session().limits().equals(limits()));
            expectFailure(()->other.respond(challenge.toByteArray()));check(!other.isActive());
        }
    }
    static void expectFailure(Runnable action) {try {action.run();throw new AssertionError("must reject");} catch(AuthFailure expected) { }}
    static LocalAuthEnvelope.Builder header(long session) {return LocalAuthEnvelope.newBuilder().setAuthVersion(1).setCapabilityId(2).setSchemaId(2).setRuntimeSession(RuntimeSession.newBuilder().setValue(session));}
    static ResourceLimits limits() {return ResourceLimits.newBuilder().setControlFrameBytes(16384).setChunkBytes(8192).setInflightBytes(262144).setInflightItems(32).setDecodedBytes(16384).setCollectionItems(256).setNestingDepth(16).setTransferBytes(1048576).build();}
    static AuthContext context(int role,int port,long session) {return new AuthContext(LocalEndpointRole.forNumber(role),port,session,repeated(1,16),repeated(2,16),repeated(3,32),repeated(4,32),Principal.newBuilder().setAccountId(AccountId.newBuilder().setValue(repeated(5,16))).setDeviceId(DeviceId.newBuilder().setValue(repeated(6,16))).build(),limits());}
    static ByteString repeated(int value,int count) {byte[] bytes=new byte[count];Arrays.fill(bytes,(byte)value);return ByteString.copyFrom(bytes);}
    static byte[] hex(String text) {return HexFormat.of().parseHex(text);}
    static String value(String json,String field) {var match=Pattern.compile("\""+field+"\": \"([0-9a-f]+)\"").matcher(json);check(match.find());return match.group(1);}
    static void check(boolean value) {if(!value) throw new AssertionError();}
}