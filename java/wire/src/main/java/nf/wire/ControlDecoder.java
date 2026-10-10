package nf.wire;

import com.google.protobuf.CodedInputStream;
import com.google.protobuf.Descriptors.Descriptor;
import com.google.protobuf.Message;
import com.google.protobuf.Parser;
import java.util.Arrays;
import nf.contract.canonical.Canonical;
import nf.contract.canonical.RecordCodec;
import org.nearfuture.protocol.v1.ControlEnvelope;
import org.nearfuture.protocol.v1.Intent;
import org.nearfuture.protocol.v1.WorldSnapshot;
import org.nearfuture.protocol.v1.SnapshotChunk;

/** Pure bounded contract admission; returns data, never authenticates or mutates world state. */
public final class ControlDecoder {
    private ControlDecoder() { }
    public enum RetryDecision { REPLAY, REQUEST_CONFLICT }
    private static <T extends Message> T decode(byte[] bytes, Descriptor descriptor, Parser<T> parser) {
        if (bytes.length > 1_048_576) throw new WireFailure(WireFailure.Code.LIMIT);
        byte[] admitted = bytes.clone();
        Preflight.inspect(admitted, descriptor);
        CodedInputStream input = CodedInputStream.newInstance(admitted);
        input.setSizeLimit(1_048_576);
        input.setRecursionLimit(32);
        try { T value = parser.parseFrom(input); SemanticAdmission.validate(value); return value; }
        catch (java.io.IOException malformed) { throw new WireFailure(WireFailure.Code.MALFORMED); }
    }
    public static ControlEnvelope decodeControl(byte[] bytes) { return decode(bytes, ControlEnvelope.getDescriptor(), ControlEnvelope.parser()); }
    /** Receive admission under already negotiated limits; legacy generic decoders retain their defaults. */
    public static ControlEnvelope decodeControl(byte[] bytes,org.nearfuture.protocol.v1.ResourceLimits limits) {
        byte[] admitted=admitControl(bytes,limits);
        int frameBytes=(int)Math.min(1_048_576,Integer.toUnsignedLong(limits.getControlFrameBytes()));
        int depth=(int)Math.min(32,Integer.toUnsignedLong(limits.getNestingDepth()));
        CodedInputStream input=CodedInputStream.newInstance(admitted);
        input.setSizeLimit(frameBytes);
        input.setRecursionLimit(depth);
        try {ControlEnvelope value=ControlEnvelope.parser().parseFrom(input);SemanticAdmission.validate(value);return value;}
        catch(java.io.IOException malformed) {throw new WireFailure(WireFailure.Code.MALFORMED);}
    }
    /** Bounded descriptor admission only; this does not authenticate or authorize an outgoing command. */
    public static void preflightControl(byte[] bytes,org.nearfuture.protocol.v1.ResourceLimits limits) {
        admitControl(bytes,limits);
    }
    private static byte[] admitControl(byte[] bytes,org.nearfuture.protocol.v1.ResourceLimits limits) {
        if(bytes==null || limits==null) throw new WireFailure(WireFailure.Code.SEMANTIC);
        int frameBytes=(int)Math.min(1_048_576,Integer.toUnsignedLong(limits.getControlFrameBytes()));
        int depth=(int)Math.min(32,Integer.toUnsignedLong(limits.getNestingDepth()));
        int items=(int)Math.min(4096,Integer.toUnsignedLong(limits.getCollectionItems()));
        long decodedBytes=Math.min(1_048_576,Integer.toUnsignedLong(limits.getDecodedBytes()));
        if(frameBytes<1 || bytes.length>frameBytes) throw new WireFailure(WireFailure.Code.LIMIT);
        byte[] admitted=bytes.clone();
        Preflight.inspectNegotiated(admitted,ControlEnvelope.getDescriptor(),depth,items,decodedBytes);
        return admitted;
    }
    public static WorldSnapshot decodeSnapshot(byte[] bytes) { return decode(bytes, WorldSnapshot.getDescriptor(), WorldSnapshot.parser()); }
    public static SnapshotChunk decodeSnapshotChunk(byte[] bytes) { return decode(bytes, SnapshotChunk.getDescriptor(), SnapshotChunk.parser()); }
    public static ControlEnvelope decodeFrame(byte[] bytes) {
        if (bytes.length < 4) throw new WireFailure(WireFailure.Code.MALFORMED);
        long length = 0;
        for (int i = 0; i < 4; i++) length = (length << 8) | (bytes[i] & 255);
        if (length == 0) throw new WireFailure(WireFailure.Code.MALFORMED);
        if (length > 1_048_576) throw new WireFailure(WireFailure.Code.LIMIT);
        if (length != bytes.length - 4L) throw new WireFailure(WireFailure.Code.MALFORMED);
        return decodeControl(Arrays.copyOfRange(bytes, 4, bytes.length));
    }
    public static byte[] canonicalIntent(Intent intent) { SemanticAdmission.validate(intent); return SemanticAdmission.canonical(intent); }
    public static byte[] bindingDigest(Intent intent) {
        SemanticAdmission.validate(intent);
        return Canonical.sha256(RecordCodec.encode(ProtoRecords.binding(intent)));
    }
    public static RetryDecision compareRetry(byte[] storedBindingDigest, byte[] storedIntentDigest, Intent candidate) {
        if (storedBindingDigest.length != 32 || storedIntentDigest.length != 32) throw new WireFailure(WireFailure.Code.SEMANTIC);
        byte[] candidateBinding = bindingDigest(candidate), candidateIntent = Canonical.sha256(canonicalIntent(candidate));
        return Arrays.equals(storedBindingDigest, candidateBinding) && Arrays.equals(storedIntentDigest, candidateIntent) ? RetryDecision.REPLAY : RetryDecision.REQUEST_CONFLICT;
    }
}
