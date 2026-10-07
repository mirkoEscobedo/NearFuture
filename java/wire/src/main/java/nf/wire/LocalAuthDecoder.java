package nf.wire;
import com.google.protobuf.CodedInputStream;
import org.nearfuture.ipc.v1.LocalAuthEnvelope;
/** Strict shape admission only; proof authentication is a separate stateful IPC client/server policy. */
public final class LocalAuthDecoder {
    private LocalAuthDecoder() { }
    public static LocalAuthEnvelope decode(byte[] bytes) {
        if (bytes==null) throw new WireFailure(WireFailure.Code.MALFORMED);
        if (bytes.length>4096) throw new WireFailure(WireFailure.Code.LIMIT);
        byte[] input=bytes.clone();Preflight.inspectAuth(input,LocalAuthEnvelope.getDescriptor());
        CodedInputStream parser=CodedInputStream.newInstance(input);parser.setSizeLimit(4096);parser.setRecursionLimit(8);
        try {LocalAuthEnvelope envelope=LocalAuthEnvelope.parseFrom(parser);AuthAdmission.validate(envelope);return envelope;}
        catch (java.io.IOException malformed) {throw new WireFailure(WireFailure.Code.MALFORMED);}
    }
    public static LocalAuthEnvelope decodeFrame(byte[] bytes) {
        if (bytes==null||bytes.length<4) throw new WireFailure(WireFailure.Code.MALFORMED);
        long length=0;for (int n=0;n<4;n++) length=(length<<8)|(bytes[n]&255);
        if (length>4096) throw new WireFailure(WireFailure.Code.LIMIT);
        if (length==0||length!=bytes.length-4L) throw new WireFailure(WireFailure.Code.MALFORMED);
        return decode(java.util.Arrays.copyOfRange(bytes,4,bytes.length));
    }
}
