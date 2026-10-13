package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import nf.wire.ChatStatusAdmission;
import nf.wire.ControlDecoder;
import nf.wire.WireFailure;
import org.nearfuture.ipc.v1.LocalEndpointRole;
import org.nearfuture.protocol.v1.*;

/** One bounded background query under an already mutually authenticated local control session. */
public final class ChatOutgoingClient {
    private final MutualAuthClient.Session session;
    private final SessionFence fence;
    private QueryChatOutgoing outstanding;
    public ChatOutgoingClient(MutualAuthClient.Session session,SessionFence fence) {
        if(session==null || fence==null || session.context().role()!=LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL) invalid();
        this.session=session;this.fence=fence;current();
    }
    public byte[] query(byte[] request,byte[] original,byte[] message) {
        current();if(outstanding!=null) throw new AuthFailure(AuthFailure.Code.STATE);
        QueryChatOutgoing q=QueryChatOutgoing.newBuilder()
                .setRequestId(RequestId.newBuilder().setValue(ByteString.copyFrom(request)))
                .setPrincipal(session.context().principal())
                .setUniverseId(UniverseId.newBuilder().setValue(session.context().universe()))
                .setHistoryId(HistoryId.newBuilder().setValue(session.context().history()))
                .setOriginalRequestId(RequestId.newBuilder().setValue(ByteString.copyFrom(original)))
                .setMessageId(ChatMessageId.newBuilder().setValue(ByteString.copyFrom(message))).build();
        ChatStatusAdmission.query(q);
        byte[] encoded=ControlEnvelope.newBuilder().setProtocolVersion(1)
                .setRuntimeSession(RuntimeSession.newBuilder().setValue(session.context().runtimeSession()))
                .setRequired(RequiredSemantics.newBuilder().addCapabilityIds(3).addSchemaIds(3))
                .setQueryChatOutgoing(q).build().toByteArray();
        if(encoded.length>Math.min(4096,Integer.toUnsignedLong(session.limits().getControlFrameBytes()))) invalid();
        outstanding=q;return encoded;
    }
    public ChatOutgoingValue accept(byte[] bytes) {
        current();if(outstanding==null) throw new AuthFailure(AuthFailure.Code.STATE);
        if(bytes==null || bytes.length==0 || bytes.length>Math.min(4096,Integer.toUnsignedLong(session.limits().getControlFrameBytes()))
                || bytes.length>Integer.toUnsignedLong(session.limits().getDecodedBytes())) invalid();
        ControlEnvelope e=ControlDecoder.decodeControl(bytes,session.limits());
        if(e.getRuntimeSession().getValue()!=session.context().runtimeSession() || !e.hasChatOutgoingStatus()) invalid();
        ChatStatusAdmission.correlate(e.getChatOutgoingStatus(),outstanding);
        current();
        outstanding=null;
        return new ChatOutgoingValue(e.getChatOutgoingStatus());
    }
    private void current() {
        if(!fence.accepts(session.context().runtimeSession())) throw new AuthFailure(AuthFailure.Code.STATE);
    }
    private static void invalid() {throw new WireFailure(WireFailure.Code.SEMANTIC);}
}
