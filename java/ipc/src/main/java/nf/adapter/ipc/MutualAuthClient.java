package nf.adapter.ipc;
import com.google.protobuf.ByteString;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.Arrays;
import nf.wire.LocalAuthDecoder;
import nf.wire.WireFailure;
import org.nearfuture.ipc.v1.*;
import org.nearfuture.protocol.v1.*;
/** Background-owner handshake state; activation requires both server proofs. No socket/game effects. */
public final class MutualAuthClient implements AutoCloseable {
    private enum State { HELLO, PROOF, ACTIVE, CLOSED }
    public static final class Session {
        private final AuthContext context;
        private final ResourceLimits limits;
        private Session(AuthContext context,ResourceLimits limits) {this.context=context;this.limits=limits;}
        public AuthContext context() {return context;}
        public ResourceLimits limits() {return limits;}
    }
    private final AuthContext context;
    private final ResourceLimits offered;
    private final byte[] token,clientNonce=new byte[32];
    private byte[] serverNonce;
    private ResourceLimits selected;
    private State state=State.HELLO;
    public MutualAuthClient(AuthContext context,byte[] token,ResourceLimits offered) {
        if(context==null||token==null||token.length!=32) AuthLimits.fail();AuthLimits.validate(offered);
        this.context=context;this.token=token.clone();this.offered=offered;new SecureRandom().nextBytes(clientNonce);
    }
    public synchronized LocalAuthEnvelope hello() {
        require(State.HELLO);
        return header().setHello(LocalAuthHello.newBuilder().setClientNonce(ByteString.copyFrom(clientNonce)).setEndpointRole(context.role()).setOfferedLimits(offered).setPrincipal(context.principal()).setProtocols(ProtocolRange.newBuilder().setMinimum(1).setMaximum(1))).build();
    }
    public synchronized LocalAuthEnvelope respond(byte[] encodedChallenge) {
        require(State.HELLO);
        try {
            var message=decode(encodedChallenge);if(!message.hasChallenge()) reject(AuthFailure.Code.PROTOCOL);
            var challenge=message.getChallenge();serverNonce=challenge.getServerNonce().toByteArray();selected=challenge.getSelectedLimits();
            if(!AuthLimits.minimum(offered,context.localLimits()).equals(selected)) reject(AuthFailure.Code.PROTOCOL);
            verify(challenge.getServerProof().toByteArray(),1);byte[] proof=proof(2);state=State.PROOF;
            return header().setProof(LocalAuthProof.newBuilder().setClientProof(ByteString.copyFrom(proof))).build();
        } catch(AuthFailure rejected) {close();throw rejected;}
    }
    public synchronized void finish(byte[] encodedAccepted) {
        require(State.PROOF);
        try {var message=decode(encodedAccepted);if(!message.hasAccepted()) reject(AuthFailure.Code.PROTOCOL);verify(message.getAccepted().getServerFinishedProof().toByteArray(),3);state=State.ACTIVE;Arrays.fill(token,(byte)0);}
        catch(AuthFailure rejected) {close();throw rejected;}
    }
    public synchronized boolean isActive() {return state==State.ACTIVE;}
    public synchronized Session session() {require(State.ACTIVE);return new Session(context,selected);}
    private LocalAuthEnvelope decode(byte[] bytes) {
        try {var message=LocalAuthDecoder.decode(bytes);if(message.getRuntimeSession().getValue()!=context.runtimeSession()) reject(AuthFailure.Code.PROTOCOL);return message;}
        catch(WireFailure rejected) {throw new AuthFailure(AuthFailure.Code.PROTOCOL);}
    }
    private void verify(byte[] actual,int stage) {if(!MessageDigest.isEqual(actual,proof(stage))) reject(AuthFailure.Code.AUTHENTICATION);}
    private byte[] proof(int stage) {return AuthTranscript.proof(token,AuthTranscript.encode(context,offered,selected,clientNonce,serverNonce,stage));}
    private LocalAuthEnvelope.Builder header() {return LocalAuthEnvelope.newBuilder().setAuthVersion(1).setCapabilityId(2).setSchemaId(2).setRuntimeSession(RuntimeSession.newBuilder().setValue(context.runtimeSession()));}
    private void require(State expected) {if(state!=expected) {close();throw new AuthFailure(AuthFailure.Code.STATE);}}
    private static void reject(AuthFailure.Code code) {throw new AuthFailure(code);}
    @Override public synchronized void close() {state=State.CLOSED;Arrays.fill(token,(byte)0);Arrays.fill(clientNonce,(byte)0);if(serverNonce!=null) Arrays.fill(serverNonce,(byte)0);}
}