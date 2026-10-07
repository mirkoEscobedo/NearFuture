package nf.wire;
import com.google.protobuf.ByteString;

import java.util.HexFormat;
import org.nearfuture.ipc.v1.*;
import org.nearfuture.protocol.v1.*;
public final class AuthWireTest {
    private AuthWireTest() { }
    public static void main(String[] arguments) throws java.io.IOException {
        LocalAuthEnvelope proof=header().setProof(LocalAuthProof.newBuilder().setClientProof(ByteString.copyFrom(new byte[32]))).build();
        if (!LocalAuthDecoder.decode(proof.toByteArray()).equals(proof)) throw new AssertionError("separate auth shape");
        ResourceLimits limits=ResourceLimits.newBuilder().setControlFrameBytes(16384).setChunkBytes(8192).setInflightBytes(262144).setInflightItems(32).setDecodedBytes(16384).setCollectionItems(256).setNestingDepth(16).setTransferBytes(1048576).build();
        Principal principal=Principal.newBuilder().setAccountId(AccountId.newBuilder().setValue(ByteString.copyFrom(new byte[16]))).setDeviceId(DeviceId.newBuilder().setValue(ByteString.copyFrom(new byte[16]))).build();
        var hello=LocalAuthHello.newBuilder().setClientNonce(ByteString.copyFrom(new byte[32])).setEndpointRole(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL).setOfferedLimits(limits).setPrincipal(principal).setProtocols(ProtocolRange.newBuilder().setMinimum(1).setMaximum(1)).build();
        var challenge=LocalAuthChallenge.newBuilder().setServerNonce(ByteString.copyFrom(new byte[32])).setSelectedLimits(limits).setServerProof(ByteString.copyFrom(new byte[32])).build();
        for(var valid:java.util.List.of(header().setHello(hello).build(),header().setChallenge(challenge).build(),header().setAccepted(LocalAuthAccepted.newBuilder().setServerFinishedProof(ByteString.copyFrom(new byte[32]))).build())) {
            if(!LocalAuthDecoder.decode(valid.toByteArray()).equals(valid)) throw new AssertionError("positive shape");
        }
        expect(WireFailure.Code.SEMANTIC,header().setHello(hello.toBuilder().clearPrincipal()).build().toByteArray());
        expect(WireFailure.Code.SEMANTIC,header().setHello(hello.toBuilder().setPrincipal(principal.toBuilder().setAccountId(AccountId.newBuilder().setValue(ByteString.copyFrom(new byte[15]))))).build().toByteArray());
        expect(WireFailure.Code.SEMANTIC,header().setHello(hello.toBuilder().setEndpointRoleValue(3)).build().toByteArray());
        expect(WireFailure.Code.UNSUPPORTED,header().setHello(hello.toBuilder().setProtocols(ProtocolRange.newBuilder().setMinimum(1).setMaximum(2))).build().toByteArray());
        expect(WireFailure.Code.LIMIT,header().setHello(hello.toBuilder().setOfferedLimits(limits.toBuilder().setInflightItems(257))).build().toByteArray());
        expect(WireFailure.Code.SEMANTIC,header().setChallenge(challenge.toBuilder().setSelectedLimits(limits.toBuilder().setDecodedBytes(1))).build().toByteArray());
        expect(WireFailure.Code.SEMANTIC,header().clearRuntimeSession().setProof(proof.getProof()).build().toByteArray());
        expect(WireFailure.Code.UNKNOWN_FIELD,java.util.HexFormat.of().parseHex("a00601"));
        expect(WireFailure.Code.MALFORMED,java.util.HexFormat.of().parseHex("088100"));
        byte[] duplicateBody=java.util.Arrays.copyOf(proof.toByteArray(),proof.toByteArray().length+2);duplicateBody[duplicateBody.length-2]=0x62;expect(WireFailure.Code.DUPLICATE,duplicateBody);
        byte[] truncated=HexFormat.of().parseHex("080110021802220909090000000000000062020a21");
        expect(WireFailure.Code.LIMIT,truncated);
        byte[] duplicate=HexFormat.of().parseHex("08010801");expect(WireFailure.Code.DUPLICATE,duplicate);
        expect(WireFailure.Code.SEMANTIC,header().setProof(LocalAuthProof.newBuilder().setClientProof(ByteString.copyFrom(new byte[31]))).build().toByteArray());
        expect(WireFailure.Code.UNSUPPORTED,proof.toBuilder().setCapabilityId(1).build().toByteArray());
        expect(WireFailure.Code.LIMIT,new byte[4097]);
        try {ControlDecoder.decodeControl(proof.toByteArray());throw new AssertionError("auth envelope cannot widen foundation");} catch (WireFailure expected) { }
        AuthWireCorpus.run();System.out.println("PASS: isolated auth wire admission and borrowed nonce/proof caps");
    }
    public static LocalAuthEnvelope.Builder header() {
        return LocalAuthEnvelope.newBuilder().setAuthVersion(1).setCapabilityId(2).setSchemaId(2).setRuntimeSession(RuntimeSession.newBuilder().setValue(9));
    }
    static void expect(WireFailure.Code code,byte[] bytes) {
        try {LocalAuthDecoder.decode(bytes);throw new AssertionError("expected "+code);}
        catch (WireFailure rejected) {if (rejected.code()!=code) throw new AssertionError(rejected.code());}
    }
}
