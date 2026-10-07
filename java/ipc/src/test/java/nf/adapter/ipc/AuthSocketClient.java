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
/** Owned headless test only; it neither discovers nor certifies native adapter/private file access. */
public final class AuthSocketClient {
    private AuthSocketClient() { }
    public static void main(String[] args) {
        try {run(args);System.out.println("AUTH_SOCKET_OK");}
        catch(Exception failure) {System.err.println("AUTH_SOCKET_FAILED");System.exit(1);}
    }
    private static void run(String[] args) throws Exception {
        if(args.length<1||args.length>2) throw new AuthFailure(AuthFailure.Code.INVALID);
        LocalEndpointRole role=args.length==1||args[1].equals("control")?LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL:args[1].equals("bulk")?LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK:null;
        if(role==null) throw new AuthFailure(AuthFailure.Code.INVALID);
        byte[] record;
        try(InputStream input=Files.newInputStream(Path.of(args[0]))) {record=input.readNBytes(265);}
        byte[] token=new byte[32];
        try {
            if(record.length!=264) throw new AuthFailure(AuthFailure.Code.INVALID);
            ByteBuffer buffer=ByteBuffer.wrap(record).order(ByteOrder.LITTLE_ENDIAN);
            magic(buffer,"NF-BLOB-1\0");if(buffer.getInt()!=218) throw new AuthFailure(AuthFailure.Code.INVALID);
            MessageDigest hash=MessageDigest.getInstance("SHA-256");hash.update(record,0,232);
            if(!MessageDigest.isEqual(hash.digest(),Arrays.copyOfRange(record,232,264))) throw new AuthFailure(AuthFailure.Code.INVALID);
            magic(buffer,"NF-IPC-R2\0");int controlPort=Short.toUnsignedInt(buffer.getShort()),bulkPort=Short.toUnsignedInt(buffer.getShort());long session=buffer.getLong();
            ByteString universe=bytes(buffer,16),history=bytes(buffer,16),ruleset=bytes(buffer,32),content=bytes(buffer,32),account=bytes(buffer,16),device=bytes(buffer,16);buffer.get(token);
            ResourceLimits limits=ResourceLimits.newBuilder().setControlFrameBytes(buffer.getInt()).setChunkBytes(buffer.getInt()).setInflightBytes(buffer.getInt()).setInflightItems(buffer.getInt()).setDecodedBytes(buffer.getInt()).setCollectionItems(buffer.getInt()).setNestingDepth(buffer.getInt()).setTransferBytes(buffer.getLong()).build();
            int port=role==LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL?controlPort:bulkPort;
            Principal principal=Principal.newBuilder().setAccountId(AccountId.newBuilder().setValue(account)).setDeviceId(DeviceId.newBuilder().setValue(device)).build();
            AuthContext context=new AuthContext(role,port,session,universe,history,ruleset,content,principal,limits);
            try(MutualAuthClient client=new MutualAuthClient(context,token,limits);BackgroundChannel channel=new BackgroundChannel(new InetSocketAddress(InetAddress.getByAddress(new byte[]{127,0,0,1}),port),4096)) {
                Arrays.fill(token,(byte)0);Arrays.fill(record,(byte)0);
                long deadline=System.nanoTime()+3_000_000_000L;
                if(!channel.send(client.hello().toByteArray())) throw new AuthFailure(AuthFailure.Code.STATE);
                if(!channel.send(client.respond(receive(channel,deadline)).toByteArray())) throw new AuthFailure(AuthFailure.Code.STATE);
                client.finish(receive(channel,deadline));if(!client.isActive()) throw new AuthFailure(AuthFailure.Code.STATE);
            }
        } finally {Arrays.fill(token,(byte)0);Arrays.fill(record,(byte)0);}
    }
    private static byte[] receive(BackgroundChannel channel,long deadline) throws Exception {
        while(System.nanoTime()-deadline<0) {byte[] message=channel.poll();if(message!=null) return message;Thread.sleep(1);}
        throw new AuthFailure(AuthFailure.Code.PROTOCOL);
    }
    private static ByteString bytes(ByteBuffer buffer,int count) {byte[] bytes=new byte[count];buffer.get(bytes);return ByteString.copyFrom(bytes);}
    private static void magic(ByteBuffer buffer,String expected) {byte[] bytes=new byte[expected.length()];buffer.get(bytes);if(!Arrays.equals(bytes,expected.getBytes(StandardCharsets.US_ASCII))) throw new AuthFailure(AuthFailure.Code.INVALID);}
}