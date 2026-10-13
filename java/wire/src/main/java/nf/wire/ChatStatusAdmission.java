package nf.wire;

import com.google.protobuf.Message;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;
import nf.contract.canonical.Canonical;
import org.nearfuture.protocol.v1.*;

/** Closed readonly Chat values. Shape/correlation is not signature authority or a financial profile. */
public final class ChatStatusAdmission {
    private ChatStatusAdmission() { }
    static void required(Message value) {
        if (!value.getUnknownFields().asMap().isEmpty()) fail();
        if (!SemanticAdmission.field(value,"capability_ids").equals(List.of(3))
                || !SemanticAdmission.field(value,"schema_ids").equals(List.of(3))) {
            throw new WireFailure(WireFailure.Code.UNSUPPORTED);
        }
    }
    private static void id(byte[] bytes) {
        if (bytes.length!=16 || Arrays.equals(bytes,new byte[16])) fail();
    }
    public static void query(QueryChatOutgoing q) {
        if (!q.hasRequestId() || !q.hasPrincipal() || !q.hasUniverseId() || !q.hasHistoryId()
                || !q.hasOriginalRequestId() || !q.hasMessageId()
                || !q.getPrincipal().hasAccountId() || !q.getPrincipal().hasDeviceId()) fail();
        id(q.getRequestId().getValue().toByteArray());
        id(q.getPrincipal().getAccountId().getValue().toByteArray());
        id(q.getPrincipal().getDeviceId().getValue().toByteArray());
        id(q.getUniverseId().getValue().toByteArray());id(q.getHistoryId().getValue().toByteArray());
        id(q.getOriginalRequestId().getValue().toByteArray());id(q.getMessageId().getValue().toByteArray());
    }
    public static void status(ChatOutgoingStatus s) {
        query(QueryChatOutgoing.newBuilder().setRequestId(s.getRequestId()).setPrincipal(s.getPrincipal())
                .setUniverseId(s.getUniverseId()).setHistoryId(s.getHistoryId())
                .setOriginalRequestId(s.getOriginalRequestId()).setMessageId(s.getMessageId()).build());
        if (s.getSourceSequence()==0 || Long.compareUnsigned(s.getOutboxRevision(),1)<0
                || Long.compareUnsigned(s.getOutboxRevision(),8192)>0) fail();
        byte[] signed=s.getSignedMessage().toByteArray();
        if (signed.length<174 || signed.length>2221 || !starts(signed,"NF-CHAT-MESSAGE-1\0")) fail();
        byte[] receipt=s.getReceiverReceipt().toByteArray();
        switch(s.getPhase()) {
            case CHAT_OUTGOING_PHASE_PENDING -> {if(receipt.length!=0) fail();}
            case CHAT_OUTGOING_PHASE_DELIVERED -> {if(receipt.length!=323 || !starts(receipt,"NF-CHAT-RECEIPT-1\0")) fail();}
            default -> throw new WireFailure(WireFailure.Code.UNSUPPORTED);
        }
    }
    /** Checks canonical original and receipt binding under a token-authenticated local report. */
    public static ChatOutgoingStatus correlate(ChatOutgoingStatus s,QueryChatOutgoing q) {
        query(q);status(s);
        if (!s.getRequestId().equals(q.getRequestId()) || !s.getPrincipal().equals(q.getPrincipal())
                || !s.getUniverseId().equals(q.getUniverseId()) || !s.getHistoryId().equals(q.getHistoryId())
                || !s.getOriginalRequestId().equals(q.getOriginalRequestId()) || !s.getMessageId().equals(q.getMessageId())) fail();
        byte[] signed=s.getSignedMessage().toByteArray();
        same(signed,18,q.getUniverseId().getValue().toByteArray());
        same(signed,34,q.getHistoryId().getValue().toByteArray());
        if(signed[50]!=1) fail();
        same(signed,51,q.getPrincipal().getAccountId().getValue().toByteArray());
        same(signed,67,q.getPrincipal().getDeviceId().getValue().toByteArray());
        same(signed,83,q.getMessageId().getValue().toByteArray());
        if(number(signed,99)!=s.getSourceSequence()) fail();
        int length=Short.toUnsignedInt(ByteBuffer.wrap(signed,107,2).order(ByteOrder.BIG_ENDIAN).getShort());
        if(length<1 || length>2048 || 109+length+64!=signed.length) fail();
        byte[] text=Arrays.copyOfRange(signed,109,109+length);
        if(!Arrays.equals(text,new String(text,StandardCharsets.UTF_8).getBytes(StandardCharsets.UTF_8))) fail();
        if(s.getPhase()==ChatOutgoingPhase.CHAT_OUTGOING_PHASE_DELIVERED) {
            byte[] r=s.getReceiverReceipt().toByteArray();
            byte[] policy=ByteBuffer.allocate(58).order(ByteOrder.BIG_ENDIAN)
                .put("NF-CHAT-POLICY-1\0".getBytes(StandardCharsets.US_ASCII))
                .put(q.getUniverseId().getValue().toByteArray()).put(q.getHistoryId().getValue().toByteArray())
                .put((byte)1).putShort((short)2048).putShort((short)4096).putShort((short)16384).putShort((short)64).array();
            same(r,18,Canonical.sha256(policy));same(r,50,q.getUniverseId().getValue().toByteArray());
            same(r,66,q.getHistoryId().getValue().toByteArray());if(r[82]!=1) fail();
            id(Arrays.copyOfRange(r,83,99));id(Arrays.copyOfRange(r,99,115));
            same(r,147,q.getOriginalRequestId().getValue().toByteArray());same(r,163,q.getMessageId().getValue().toByteArray());
            same(r,179,q.getPrincipal().getAccountId().getValue().toByteArray());
            same(r,195,q.getPrincipal().getDeviceId().getValue().toByteArray());
            if(number(r,211)!=s.getSourceSequence() || number(r,219)==0) fail();
            byte[] domain="NF-CHAT-SIGNED-MESSAGE-1\0".getBytes(StandardCharsets.US_ASCII);
            byte[] original=new byte[domain.length+signed.length];
            System.arraycopy(domain,0,original,0,domain.length);System.arraycopy(signed,0,original,domain.length,signed.length);
            same(r,227,Canonical.sha256(original));
        }
        return s;
    }
    private static boolean starts(byte[] bytes,String domain) {
        return Arrays.equals(Arrays.copyOfRange(bytes,0,domain.length()),domain.getBytes(StandardCharsets.US_ASCII));
    }
    private static long number(byte[] bytes,int at) {return ByteBuffer.wrap(bytes,at,8).order(ByteOrder.BIG_ENDIAN).getLong();}
    private static void same(byte[] bytes,int at,byte[] expected) {
        if(at+expected.length>bytes.length || !Arrays.equals(Arrays.copyOfRange(bytes,at,at+expected.length),expected)) fail();
    }
    private static void fail() {throw new WireFailure(WireFailure.Code.SEMANTIC);}
}
