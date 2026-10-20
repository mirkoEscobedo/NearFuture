package nf.wire;

import com.google.protobuf.ByteString;
import com.google.protobuf.UnknownFieldSet;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HexFormat;
import org.nearfuture.protocol.v1.*;

/** Pure reply admission/correlation controls; zero signature or authorization authority. */
public final class ChatEnqueueReplyAdmissionTest {
    private ChatEnqueueReplyAdmissionTest() { }
    private static ByteString id(int value) {
        byte[] bytes = new byte[16]; Arrays.fill(bytes, (byte) value); return ByteString.copyFrom(bytes);
    }
    private static RequiredSemantics profile(int value) {
        return RequiredSemantics.newBuilder().addCapabilityIds(value).addSchemaIds(value).build();
    }
    private static UnknownFieldSet unknown() {
        return UnknownFieldSet.newBuilder().addField(99,
                UnknownFieldSet.Field.newBuilder().addVarint(1).build()).build();
    }
    private static ChatOutgoingStatus pending() {
        Principal principal = Principal.newBuilder().setAccountId(AccountId.newBuilder().setValue(id(22)))
                .setDeviceId(DeviceId.newBuilder().setValue(id(33))).build();
        byte[] signed = ByteBuffer.allocate(174)
                .put("NF-CHAT-MESSAGE-1\0".getBytes(StandardCharsets.US_ASCII))
                .put(id(11).toByteArray()).put(id(12).toByteArray()).put((byte) 1)
                .put(id(22).toByteArray()).put(id(33).toByteArray()).put(id(81).toByteArray())
                .putLong(1).putShort((short) 1).put((byte) 'x').put(new byte[64]).array();
        return ChatOutgoingStatus.newBuilder().setRequestId(RequestId.newBuilder().setValue(id(111)))
                .setPrincipal(principal).setUniverseId(UniverseId.newBuilder().setValue(id(11)))
                .setHistoryId(HistoryId.newBuilder().setValue(id(12)))
                .setOriginalRequestId(RequestId.newBuilder().setValue(id(101)))
                .setMessageId(ChatMessageId.newBuilder().setValue(id(81)))
                .setSourceSequence(1).setOutboxRevision(1)
                .setPhase(ChatOutgoingPhase.CHAT_OUTGOING_PHASE_PENDING)
                .setSignedMessage(ByteString.copyFrom(signed)).build();
    }
    private static QueryChatOutgoing query(ChatOutgoingStatus status) {
        return QueryChatOutgoing.newBuilder().setRequestId(status.getRequestId()).setPrincipal(status.getPrincipal())
                .setUniverseId(status.getUniverseId()).setHistoryId(status.getHistoryId())
                .setOriginalRequestId(status.getOriginalRequestId()).setMessageId(status.getMessageId()).build();
    }
    private static ControlEnvelope reply(ChatOutgoingStatus status) {
        return ControlEnvelope.newBuilder().setProtocolVersion(1)
                .setRuntimeSession(RuntimeSession.newBuilder().setValue(7)).setRequired(profile(4))
                .setChatEnqueueResult(ChatEnqueueResult.newBuilder().setStatus(status)).build();
    }
    private static void rejected(String label, WireFailure.Code code, Runnable action) {
        try { action.run(); throw new AssertionError(label + " accepted"); }
        catch (WireFailure actual) {
            if (actual.code() != code) throw new AssertionError(label + " expected " + code + " actual " + actual.code());
        }
    }
    private static void decodeRejected(String label, WireFailure.Code code, ControlEnvelope value) {
        rejected(label, code, () -> ControlDecoder.decodeControl(value.toByteArray()));
    }
    public static void main(String[] args) {
        ChatOutgoingStatus status = pending(); ControlEnvelope accepted = reply(status);
        if (!ControlDecoder.decodeControl(accepted.toByteArray()).equals(accepted)) throw new AssertionError("profile4 reply changed");
        if (!ChatStatusAdmission.correlate(status, query(status)).equals(status)) throw new AssertionError("canonical Pending changed");
        System.out.println("PASS: positive profile4 reply and independently laid-out canonical Pending");

        for (RequiredSemantics bad : new RequiredSemantics[] {RequiredSemantics.getDefaultInstance(), profile(1), profile(3),
                profile(4).toBuilder().clearSchemaIds().build(), profile(4).toBuilder().addCapabilityIds(4).build(),
                profile(4).toBuilder().addSchemaIds(5).build()}) {
            decodeRejected("closed exact4", WireFailure.Code.UNSUPPORTED, accepted.toBuilder().setRequired(bad).build());
        }
        System.out.println("PASS: empty, wrong, missing, duplicate and expanded reply profiles reject");

        ControlEnvelope generic = ControlDecoder.decodeControl(HexFormat.of().parseHex(WireCorpus.POSITIVE.get(0).hex()));
        if (!ControlDecoder.decodeControl(generic.toByteArray()).equals(generic)) throw new AssertionError("generic profile changed");
        decodeRejected("profile4 generic leakage", WireFailure.Code.UNSUPPORTED, generic.toBuilder().setRequired(profile(4)).build());
        ControlEnvelope readonly = accepted.toBuilder().clearChatEnqueueResult().setRequired(profile(3)).setChatOutgoingStatus(status).build();
        if (!ControlDecoder.decodeControl(readonly.toByteArray()).equals(readonly)) throw new AssertionError("readonly3 changed");
        decodeRejected("profile4 readonly leakage", WireFailure.Code.UNSUPPORTED, readonly.toBuilder().setRequired(profile(4)).build());
        decodeRejected("no outgoing command grant", WireFailure.Code.UNSUPPORTED,
                accepted.toBuilder().clearChatEnqueueResult().setEnqueueChat(EnqueueChat.getDefaultInstance()).build());
        System.out.println("PASS: existing generic/readonly admission retained and profile4 stays reply-only");

        decodeRejected("unknown envelope", WireFailure.Code.UNKNOWN_FIELD, accepted.toBuilder().setUnknownFields(unknown()).build());
        decodeRejected("unknown required", WireFailure.Code.UNKNOWN_FIELD,
                accepted.toBuilder().setRequired(profile(4).toBuilder().setUnknownFields(unknown())).build());
        decodeRejected("unknown result", WireFailure.Code.UNKNOWN_FIELD,
                accepted.toBuilder().setChatEnqueueResult(accepted.getChatEnqueueResult().toBuilder().setUnknownFields(unknown())).build());
        decodeRejected("unknown status", WireFailure.Code.UNKNOWN_FIELD, reply(status.toBuilder().setUnknownFields(unknown()).build()));
        decodeRejected("missing status", WireFailure.Code.SEMANTIC,
                accepted.toBuilder().setChatEnqueueResult(ChatEnqueueResult.getDefaultInstance()).build());
        System.out.println("PASS: unknown fields at all four reply levels and missing status reject");

        decodeRejected("zero revision", WireFailure.Code.SEMANTIC, reply(status.toBuilder().setOutboxRevision(0).build()));
        decodeRejected("revision above retained ceiling", WireFailure.Code.SEMANTIC, reply(status.toBuilder().setOutboxRevision(8193).build()));
        decodeRejected("unknown phase", WireFailure.Code.UNSUPPORTED, reply(status.toBuilder().setPhaseValue(9).build()));
        decodeRejected("Pending receipt", WireFailure.Code.SEMANTIC, reply(status.toBuilder().setReceiverReceipt(ByteString.copyFrom(new byte[323])).build()));
        decodeRejected("Delivered missing receipt", WireFailure.Code.SEMANTIC, reply(status.toBuilder().setPhase(ChatOutgoingPhase.CHAT_OUTGOING_PHASE_DELIVERED).build()));
        byte[] wrongDomain = status.getSignedMessage().toByteArray(); wrongDomain[0] ^= 1;
        decodeRejected("canonical domain", WireFailure.Code.SEMANTIC, reply(status.toBuilder().setSignedMessage(ByteString.copyFrom(wrongDomain)).build()));
        System.out.println("PASS: retained revision, phase, receipt and signed-domain admission rejects");

        ChatOutgoingStatus changedRequest = status.toBuilder().setRequestId(RequestId.newBuilder().setValue(id(112))).build();
        ControlDecoder.decodeControl(reply(changedRequest).toByteArray());
        rejected("trusted request correlation", WireFailure.Code.SEMANTIC, () -> ChatStatusAdmission.correlate(changedRequest, query(status)));
        byte[] wrongSequence = status.getSignedMessage().toByteArray(); wrongSequence[106] = 2;
        ChatOutgoingStatus changedCanonical = status.toBuilder().setSignedMessage(ByteString.copyFrom(wrongSequence)).build();
        ControlDecoder.decodeControl(reply(changedCanonical).toByteArray());
        rejected("canonical source correlation", WireFailure.Code.SEMANTIC, () -> ChatStatusAdmission.correlate(changedCanonical, query(status)));
        byte[] wrongLength = status.getSignedMessage().toByteArray(); wrongLength[108] = 2;
        rejected("canonical declared text", WireFailure.Code.SEMANTIC, () -> ChatStatusAdmission.correlate(
                status.toBuilder().setSignedMessage(ByteString.copyFrom(wrongLength)).build(), query(status)));
        System.out.println("PASS: shape admission does not bypass retained trusted/canonical correlation");
    }
}
