package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import java.io.InputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.Arrays;
import org.nearfuture.ipc.v1.LocalEndpointRole;
import org.nearfuture.protocol.v1.*;

/** Owned headless fixture only. This does not certify game discovery or native adapter ACL access. */
public final class ChatStatusSocketClient {
    private ChatStatusSocketClient() { }
    public static void main(String[] args) {
        try {run(args);System.out.println("CHAT_STATUS_SOCKET_OK");}
        catch(AssertionError failure) {System.err.println(failure.getMessage());System.exit(1);}
        catch(Exception failure) {System.err.println("CHAT_STATUS_SETUP_OR_ADMISSION_FAILED:"+failure.getClass().getSimpleName());System.exit(2);}
    }
    private static void run(String[] args) throws Exception {
        if(args.length!=8) throw new AuthFailure(AuthFailure.Code.INVALID);
        byte[] query=hex(args[1],16),original=hex(args[2],16),message=hex(args[3],16);
        int phase=Integer.parseInt(args[4]);long revision=Long.parseLong(args[5]);
        if(phase<1 || phase>2 || revision<1 || revision>8192) throw new AuthFailure(AuthFailure.Code.INVALID);
        byte[] expected=hexBounded(args[6],174,2221);
        byte[] expectedReceipt=args[7].equals("-")?new byte[0]:hex(args[7],323);
        if((phase==1 && expectedReceipt.length!=0)||(phase==2 && expectedReceipt.length!=323)) throw new AuthFailure(AuthFailure.Code.INVALID);
        byte[] record;
        try(InputStream input=Files.newInputStream(Path.of(args[0]))) {record=input.readNBytes(265);}
        byte[] token=new byte[32];
        try {
            if(record.length!=264) throw new AuthFailure(AuthFailure.Code.INVALID);
            ByteBuffer buffer=ByteBuffer.wrap(record).order(ByteOrder.LITTLE_ENDIAN);
            magic(buffer,"NF-BLOB-1\0");if(buffer.getInt()!=218) throw new AuthFailure(AuthFailure.Code.INVALID);
            MessageDigest hash=MessageDigest.getInstance("SHA-256");hash.update(record,0,232);
            if(!MessageDigest.isEqual(hash.digest(),Arrays.copyOfRange(record,232,264))) throw new AuthFailure(AuthFailure.Code.INVALID);
            magic(buffer,"NF-IPC-R2\0");
            int controlPort=Short.toUnsignedInt(buffer.getShort());buffer.getShort();long runtime=buffer.getLong();
            ByteString universe=bytes(buffer,16),history=bytes(buffer,16),ruleset=bytes(buffer,32),content=bytes(buffer,32),account=bytes(buffer,16),device=bytes(buffer,16);buffer.get(token);
            ResourceLimits limits=ResourceLimits.newBuilder().setControlFrameBytes(buffer.getInt()).setChunkBytes(buffer.getInt()).setInflightBytes(buffer.getInt()).setInflightItems(buffer.getInt()).setDecodedBytes(buffer.getInt()).setCollectionItems(buffer.getInt()).setNestingDepth(buffer.getInt()).setTransferBytes(buffer.getLong()).build();
            Principal principal=Principal.newBuilder().setAccountId(AccountId.newBuilder().setValue(account)).setDeviceId(DeviceId.newBuilder().setValue(device)).build();
            AuthContext context=new AuthContext(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL,controlPort,runtime,universe,history,ruleset,content,principal,limits);
            SessionFence fence=new SessionFence(runtime);
            try(MutualAuthClient auth=new MutualAuthClient(context,token,limits);BackgroundChannel channel=new BackgroundChannel(new InetSocketAddress(InetAddress.getByAddress(new byte[]{127,0,0,1}),controlPort),4096)) {
                Arrays.fill(token,(byte)0);Arrays.fill(record,(byte)0);
                long deadline=System.nanoTime()+3_000_000_000L;
                if(!channel.send(auth.hello().toByteArray())) throw new AuthFailure(AuthFailure.Code.STATE);
                if(!channel.send(auth.respond(receive(channel,deadline)).toByteArray())) throw new AuthFailure(AuthFailure.Code.STATE);
                auth.finish(receive(channel,deadline));
                if(!auth.isActive()) throw new AuthFailure(AuthFailure.Code.STATE);
                ChatOutgoingClient client=new ChatOutgoingClient(auth.session(),fence);
                if(!channel.send(client.query(query,original,message))) throw new AuthFailure(AuthFailure.Code.STATE);
                byte[] response=receive(channel,deadline);
                ChatOutgoingValue value;
                try {value=client.accept(response);}
                catch(UnsupportedOperationException unsupported) {
                    throw new AssertionError("CHAT_FIRST_ASSERTION Unsupported != "+(phase==1?"Pending":"Delivered"));
                }
                ChatOutgoingStatus status=value.status();
                if(status.getPhaseValue()!=phase || status.getOutboxRevision()!=revision || status.getSourceSequence()!=1
                        || !Arrays.equals(status.getSignedMessage().toByteArray(),expected)
                        || !Arrays.equals(status.getReceiverReceipt().toByteArray(),expectedReceipt)) {
                    throw new AssertionError("CHAT_FIRST_ASSERTION projectedvalue != independentlyexpectedoriginal");
                }
                fence.invalidate();
                if(fence.accepts(runtime)) throw new AssertionError("CHAT_FENCE_INVALIDATION_FAILED");
            }
        } finally {Arrays.fill(token,(byte)0);Arrays.fill(record,(byte)0);}
    }
    private static byte[] receive(BackgroundChannel channel,long deadline) throws Exception {
        while(System.nanoTime()-deadline<0) {byte[] message=channel.poll();if(message!=null) return message;Thread.sleep(1);}
        throw new AuthFailure(AuthFailure.Code.PROTOCOL);
    }
    private static byte[] hex(String value,int count) {return hexBounded(value,count,count);}
    private static byte[] hexBounded(String value,int min,int max) {
        if(value.length()%2!=0 || value.length()<min*2 || value.length()>max*2) throw new AuthFailure(AuthFailure.Code.INVALID);
        byte[] bytes=new byte[value.length()/2];
        for(int i=0;i<bytes.length;i++) {
            int high=Character.digit(value.charAt(i*2),16),low=Character.digit(value.charAt(i*2+1),16);
            if(high<0 || low<0) throw new AuthFailure(AuthFailure.Code.INVALID);
            bytes[i]=(byte)(high*16+low);
        }
        return bytes;
    }
    private static ByteString bytes(ByteBuffer buffer,int count) {byte[] bytes=new byte[count];buffer.get(bytes);return ByteString.copyFrom(bytes);}
    private static void magic(ByteBuffer buffer,String expected) {byte[] bytes=new byte[expected.length()];buffer.get(bytes);if(!Arrays.equals(bytes,expected.getBytes(StandardCharsets.US_ASCII))) throw new AuthFailure(AuthFailure.Code.INVALID);}
}
