package nf.adapter.ipc;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;
import org.nearfuture.protocol.v1.ResourceLimits;
/** Fixed NF-IPC-AUTH-1 transcript. Protobuf serialization is never hashed. */
public final class AuthTranscript {
    private AuthTranscript() { }
    public static byte[] encode(AuthContext context,ResourceLimits offered,ResourceLimits selected,byte[] clientNonce,byte[] serverNonce,int stage) {
        if(context==null||stage<1||stage>3||clientNonce==null||clientNonce.length!=32||serverNonce==null||serverNonce.length!=32) AuthLimits.fail();
        if(!AuthLimits.minimum(offered,context.localLimits()).equals(selected)) AuthLimits.fail();
        ByteBuffer buffer=ByteBuffer.allocate(306).order(ByteOrder.LITTLE_ENDIAN);
        buffer.put("NF-IPC-AUTH-1\0".getBytes(StandardCharsets.US_ASCII)).put((byte)stage).putInt(1).putInt(2).putInt(2).putInt(1).put((byte)context.role().getNumber()).putShort((short)context.port()).putLong(context.runtimeSession());
        buffer.put(context.universe().toByteArray()).put(context.history().toByteArray()).put(context.ruleset().toByteArray()).put(context.content().toByteArray()).put(context.principal().getAccountId().getValue().toByteArray()).put(context.principal().getDeviceId().getValue().toByteArray()).put(clientNonce).put(serverNonce);
        append(buffer,offered);append(buffer,selected);return buffer.array();
    }
    public static byte[] proof(byte[] token,byte[] transcript) {
        if(token==null||token.length!=32||transcript==null||transcript.length!=306) AuthLimits.fail();
        try {Mac mac=Mac.getInstance("HmacSHA256");mac.init(new SecretKeySpec(token,"HmacSHA256"));return mac.doFinal(transcript);}
        catch(GeneralSecurityException unavailable) {throw new AuthFailure(AuthFailure.Code.AUTHENTICATION);}
    }
    private static void append(ByteBuffer buffer,ResourceLimits limits) {long[] values=AuthLimits.values(limits);for(int i=0;i<7;i++) buffer.putInt((int)values[i]);buffer.putLong(values[7]);}
}