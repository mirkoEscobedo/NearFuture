package nf.wire;
import com.google.protobuf.ByteString;
import org.nearfuture.ipc.v1.LocalAuthEnvelope;
final class AuthAdmission {
    private AuthAdmission() { }
    static void validate(LocalAuthEnvelope envelope) {
        if (envelope.getAuthVersion()!=1||envelope.getCapabilityId()!=2||envelope.getSchemaId()!=2) throw new WireFailure(WireFailure.Code.UNSUPPORTED);
        if (!envelope.hasRuntimeSession()||envelope.getRuntimeSession().getValue()==0) fail();
        switch (envelope.getBodyCase()) {
            case HELLO -> {
                var hello=envelope.getHello();width(hello.getClientNonce());
                if (hello.getEndpointRoleValue()<1||hello.getEndpointRoleValue()>2) fail();
                if (!hello.hasOfferedLimits()||!hello.hasPrincipal()||!hello.hasProtocols()) fail();
                SemanticAdmission.validate(hello.getOfferedLimits());SemanticAdmission.validate(hello.getPrincipal());
                if (hello.getProtocols().getMinimum()!=1||hello.getProtocols().getMaximum()!=1) throw new WireFailure(WireFailure.Code.UNSUPPORTED);
            }
            case CHALLENGE -> {
                var challenge=envelope.getChallenge();width(challenge.getServerNonce());width(challenge.getServerProof());
                if (!challenge.hasSelectedLimits()) fail();SemanticAdmission.validate(challenge.getSelectedLimits());
            }
            case PROOF -> width(envelope.getProof().getClientProof());
            case ACCEPTED -> width(envelope.getAccepted().getServerFinishedProof());
            default -> fail();
        }
    }
    private static void width(ByteString bytes) {if (bytes.size()!=32) fail();}
    private static void fail() {throw new WireFailure(WireFailure.Code.SEMANTIC);}
}
