package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import java.util.Arrays;
import nf.contract.canonical.Canonical;
import nf.wire.ChatStatusAdmission;
import nf.wire.ControlDecoder;
import nf.wire.WireFailure;
import org.nearfuture.ipc.v1.LocalEndpointRole;
import org.nearfuture.protocol.v1.*;

/** One bounded owner-local enqueue under an already mutually authenticated control session. */
public final class ChatEnqueueClient {
    private final MutualAuthClient.Session session;
    private final SessionFence fence;
    private EnqueueChat outstanding;
    private byte[] outstandingText;
    public ChatEnqueueClient(MutualAuthClient.Session session,SessionFence fence) {
        if(session==null || fence==null || session.context().role()!=LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL) invalid();
        this.session=session;this.fence=fence;current();
    }
    /** The caller supplies correlation and text; the trusted owner allocates and signs the original. */
    public byte[] enqueue(byte[] request,byte[] original,String text) {
        current();if(outstanding!=null) throw new AuthFailure(AuthFailure.Code.STATE);
        id(request);id(original);if(text==null) invalid();
        byte[] canonicalText=Canonical.validateText(text,2048);
        if(canonicalText.length<1) invalid();
        EnqueueChat command=EnqueueChat.newBuilder()
                .setRequestId(RequestId.newBuilder().setValue(ByteString.copyFrom(request)))
                .setPrincipal(session.context().principal())
                .setUniverseId(UniverseId.newBuilder().setValue(session.context().universe()))
                .setHistoryId(HistoryId.newBuilder().setValue(session.context().history()))
                .setOriginalRequestId(RequestId.newBuilder().setValue(ByteString.copyFrom(original)))
                .setChannel(1).setText(text).build();
        byte[] encoded=ControlEnvelope.newBuilder().setProtocolVersion(1)
                .setRuntimeSession(RuntimeSession.newBuilder().setValue(session.context().runtimeSession()))
                .setRequired(RequiredSemantics.newBuilder().addCapabilityIds(4).addSchemaIds(4))
                .setEnqueueChat(command).build().toByteArray();
        if(encoded.length>Math.min(4096,Integer.toUnsignedLong(session.limits().getControlFrameBytes()))
                || encoded.length>Integer.toUnsignedLong(session.limits().getDecodedBytes())) invalid();
        ControlDecoder.preflightControl(encoded,session.limits());
        current();outstanding=command;outstandingText=canonicalText;return encoded;
    }
    /** Shape and original correlation are authenticated local reports, not independent signature authority. */
    public ChatOutgoingValue accept(byte[] bytes) {
        current();if(outstanding==null) throw new AuthFailure(AuthFailure.Code.STATE);
        if(bytes==null || bytes.length==0 || bytes.length>Math.min(4096,Integer.toUnsignedLong(session.limits().getControlFrameBytes()))
                || bytes.length>Integer.toUnsignedLong(session.limits().getDecodedBytes())) invalid();
        ControlEnvelope envelope=ControlDecoder.decodeControl(bytes,session.limits());
        if(envelope.getRuntimeSession().getValue()!=session.context().runtimeSession()
                || !envelope.hasChatEnqueueResult() || !envelope.getChatEnqueueResult().hasStatus()) invalid();
        ChatOutgoingStatus status=envelope.getChatEnqueueResult().getStatus();
        ChatStatusAdmission.correlate(status,QueryChatOutgoing.newBuilder()
                .setRequestId(outstanding.getRequestId()).setPrincipal(outstanding.getPrincipal())
                .setUniverseId(outstanding.getUniverseId()).setHistoryId(outstanding.getHistoryId())
                .setOriginalRequestId(outstanding.getOriginalRequestId()).setMessageId(status.getMessageId()).build());
        byte[] signed=status.getSignedMessage().toByteArray();
        // Correlate has already checked the exact canonical body length and signature suffix shape.
        if(!Arrays.equals(outstandingText,Arrays.copyOfRange(signed,109,signed.length-64))) invalid();
        current();outstanding=null;outstandingText=null;
        return new ChatOutgoingValue(status);
    }
    private void current() {
        if(!fence.accepts(session.context().runtimeSession())) throw new AuthFailure(AuthFailure.Code.STATE);
    }
    private static void id(byte[] bytes) {
        if(bytes==null || bytes.length!=16 || Arrays.equals(bytes,new byte[16])) invalid();
    }
    private static void invalid() {throw new WireFailure(WireFailure.Code.SEMANTIC);}
}
