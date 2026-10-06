package nf.wire;

import com.google.protobuf.ByteString;
import java.util.Arrays;
import java.util.HexFormat;
import nf.contract.canonical.Canonical;
import org.nearfuture.protocol.v1.*;

public final class WirePolicyTest {
    private WirePolicyTest() { }
    private static byte[] hex(String value) { return HexFormat.of().parseHex(value); }
    private static ByteString bytes(String value) { return ByteString.copyFrom(hex(value)); }
    private static void rejected(Runnable action) { try { action.run(); throw new AssertionError("Malformed semantic input accepted"); } catch (WireFailure expected) { } }
    public static void main(String[] args) {
        String id4 = "44".repeat(16), id5 = "55".repeat(16), hash9 = "99".repeat(32);
        WorldSnapshot snapshot = WorldSnapshot.newBuilder().setUniverseId(UniverseId.newBuilder().setValue(bytes(id4)))
                .setHistoryId(HistoryId.newBuilder().setValue(bytes(id5))).setEventSeq(EventSequence.getDefaultInstance())
                .setWorldTick(WorldTick.getDefaultInstance()).setRulesetHash(Sha256Digest.newBuilder().setValue(bytes(hash9)))
                .setRequired(RequiredSemantics.getDefaultInstance())
                .setStateHash(Sha256Digest.newBuilder().setValue(bytes("37c569a430ca97c61e6ff5a0113e35154843bf44f5aec1d75eb5025eb0e031e8"))).build();
        ControlDecoder.decodeSnapshot(snapshot.toByteArray());
        rejected(() -> ControlDecoder.decodeSnapshot(snapshot.toBuilder().clearWorldTick().build().toByteArray()));
        rejected(() -> ControlDecoder.decodeSnapshot(snapshot.toBuilder().setStateHash(Sha256Digest.newBuilder().setValue(ByteString.copyFrom(new byte[32]))).build().toByteArray()));
        SnapshotChunk chunk = SnapshotChunk.newBuilder().setTransferId(OperationId.newBuilder().setValue(bytes("11".repeat(16))))
                .setSnapshotDigest(Sha256Digest.newBuilder().setValue(bytes(hash9))).setChunkCount(1).setTotalBytes(1).setData(ByteString.copyFrom(new byte[] {0})).build();
        ControlDecoder.decodeSnapshotChunk(chunk.toByteArray());
        rejected(() -> ControlDecoder.decodeSnapshotChunk(chunk.toBuilder().setChunkIndex(1).build().toByteArray()));
        rejected(() -> ControlDecoder.decodeSnapshotChunk(chunk.toBuilder().setChunkCount(257).build().toByteArray()));
        rejected(() -> ControlDecoder.decodeSnapshotChunk(chunk.toBuilder().setTotalBytes(-1L).build().toByteArray()));
        rejected(() -> ControlDecoder.decodeSnapshotChunk(chunk.toBuilder().clearData().build().toByteArray()));
        rejected(() -> ControlDecoder.decodeSnapshotChunk(chunk.toBuilder().setData(ByteString.copyFrom(new byte[] {0, 0})).build().toByteArray()));
        rejected(() -> ControlDecoder.decodeSnapshotChunk(chunk.toBuilder().setTotalBytes(262145).build().toByteArray()));
        Intent original = ControlDecoder.decodeControl(hex(WireCorpus.POSITIVE.get(2).hex())).getIntent();
        byte[] binding = ControlDecoder.bindingDigest(original), digest = Canonical.sha256(ControlDecoder.canonicalIntent(original));
        if (ControlDecoder.compareRetry(binding, digest, original) != ControlDecoder.RetryDecision.REPLAY) throw new AssertionError("Exact retry rejected");
        Intent alteredHistory = original.toBuilder().setHistoryId(HistoryId.newBuilder().setValue(bytes("66".repeat(16)))).build();
        Intent alteredAccount = original.toBuilder().setPrincipal(original.getPrincipal().toBuilder().setAccountId(AccountId.newBuilder().setValue(bytes("66".repeat(16))))).build();
        Intent alteredExpiry = original.toBuilder().setExpiresAtTick(0).build();
        Intent alteredRevisions = original.toBuilder().addExpectedRevisions(AggregateVersion.newBuilder().setAggregateId(AggregateId.newBuilder().setValue(bytes("11".repeat(16)))).setRevision(AggregateRevision.getDefaultInstance())).build();
        for (Intent altered : new Intent[] {alteredHistory, alteredAccount, alteredExpiry, alteredRevisions}) {
            if (ControlDecoder.compareRetry(binding, digest, altered) != ControlDecoder.RetryDecision.REQUEST_CONFLICT) throw new AssertionError("Changed intent interpreted as original operation");
        }
        rejected(() -> ControlDecoder.compareRetry(binding, digest, original.toBuilder().setOperationKind(2).build()));
        byte[] profileBody = hex("4e462d43414e4f4e2d3100ff00010001000001000000000000000000000000");
        SchemaPayload payload = SchemaPayload.newBuilder().setSchemaId(1).setCanonicalBody(ByteString.copyFrom(profileBody)).build();
        byte[] changedDigest = Canonical.sha256(nf.contract.canonical.RecordCodec.encode(new nf.contract.canonical.Records.Document(6, 1,
                java.util.List.of(new nf.contract.canonical.Records.U32(1), new nf.contract.canonical.Records.Bytes(profileBody)))));
        Intent changedPayload = original.toBuilder().setPayload(payload).setPayloadDigest(Sha256Digest.newBuilder().setValue(ByteString.copyFrom(changedDigest))).build();
        if (ControlDecoder.compareRetry(binding, digest, changedPayload) != ControlDecoder.RetryDecision.REQUEST_CONFLICT) throw new AssertionError("Changed payload interpreted as original");
        OperationStatus contradicted = OperationStatus.newBuilder().setRequestId(original.getRequestId()).setOperationId(OperationId.newBuilder().setValue(bytes("11".repeat(16))))
                .setHistoryId(original.getHistoryId()).setRequestBindingDigest(Sha256Digest.newBuilder().setValue(ByteString.copyFrom(binding)))
                .setPhase(OperationPhase.OPERATION_PHASE_PENDING).setSuccess(original.getPayload()).build();
        ControlEnvelope contradictoryEnvelope = ControlEnvelope.newBuilder().setProtocolVersion(1).setRuntimeSession(RuntimeSession.newBuilder().setValue(1))
                .setRequired(RequiredSemantics.getDefaultInstance()).setOperationStatus(contradicted).build();
        rejected(() -> ControlDecoder.decodeControl(contradictoryEnvelope.toByteArray()));
        ControlEnvelope handshakeEnvelope = ControlDecoder.decodeControl(hex(WireCorpus.POSITIVE.get(3).hex()));
        ResourceLimits limits = handshakeEnvelope.getHandshake().getLimits();
        NegotiatedSession negotiated = NegotiatedSession.newBuilder().setProtocolVersion(1).setSemantics(handshakeEnvelope.getRequired())
                .setLimits(limits).setRuntimeSession(handshakeEnvelope.getRuntimeSession()).build();
        ControlEnvelope accepted = handshakeEnvelope.toBuilder().clearHandshake().setHandshakeResult(HandshakeResult.newBuilder().setAccepted(negotiated)).build();
        ControlDecoder.decodeControl(accepted.toByteArray());
        rejected(() -> ControlDecoder.decodeControl(accepted.toBuilder().setHandshakeResult(HandshakeResult.newBuilder()
                .setAccepted(negotiated.toBuilder().setRuntimeSession(RuntimeSession.newBuilder().setValue(2)))).build().toByteArray()));
        rejected(() -> ControlDecoder.decodeControl(handshakeEnvelope.toBuilder().setHandshake(handshakeEnvelope.getHandshake().toBuilder()
                .setLimits(limits.toBuilder().setTransferBytes(1))).build().toByteArray()));
        byte[] seed = hex(WireCorpus.POSITIVE.get(0).hex());
        long state = 0x1270aL;
        for (int i = 0; i < 10_000; i++) {
            state ^= state << 13; state ^= state >>> 7; state ^= state << 17;
            byte[] input = seed.clone();
            int index = (int) Long.remainderUnsigned(state, input.length);
            input[index] = (byte) (state >>> 32);
            if ((state & 3) == 0) input = Arrays.copyOf(input, index);
            try { ControlDecoder.decodeControl(input); } catch (WireFailure rejected) { }
        }
        rejected(() -> ControlDecoder.decodeControl(new byte[1_048_577]));
        System.out.println("PASS: snapshot hash/presence, chunk quotas, request conflicts, phase consistency, 10000 finite wire mutations");
    }
}
