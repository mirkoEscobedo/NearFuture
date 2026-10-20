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
public final class ChatEnqueueSocketClient {
    private ChatEnqueueSocketClient() { }
    public static void main(String[] args) {
        try {run(args);}
        catch(AssertionError failure) {System.err.println(failure.getMessage());System.exit(1);}
        catch(Exception failure) {System.err.println("CHAT_ENQUEUE_SETUP_OR_ADMISSION_FAILED:"+failure.getClass().getSimpleName());System.exit(2);}
    }
    private static void run(String[] args) throws Exception {
        if(args.length!=4) throw new AuthFailure(AuthFailure.Code.INVALID);
        byte[] request=hex(args[1],16),original=hex(args[2],16);
        String text=args[3];nf.contract.canonical.Canonical.validateText(text,2048);
        if(text.getBytes(StandardCharsets.UTF_8).length<1 || text.getBytes(StandardCharsets.UTF_8).length>2048) throw new AuthFailure(AuthFailure.Code.INVALID);
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
                EnqueueChat command=EnqueueChat.newBuilder()
                        .setRequestId(RequestId.newBuilder().setValue(ByteString.copyFrom(request)))
                        .setPrincipal(context.principal()).setUniverseId(UniverseId.newBuilder().setValue(context.universe()))
                        .setHistoryId(HistoryId.newBuilder().setValue(context.history()))
                        .setOriginalRequestId(RequestId.newBuilder().setValue(ByteString.copyFrom(original)))
                        .setChannel(1).setText(text).build();
                byte[] encoded=ControlEnvelope.newBuilder().setProtocolVersion(1)
                        .setRuntimeSession(RuntimeSession.newBuilder().setValue(context.runtimeSession()))
                        .setRequired(RequiredSemantics.newBuilder().addCapabilityIds(4).addSchemaIds(4))
                        .setEnqueueChat(command).build().toByteArray();
                if(encoded.length>Math.min(4096,Integer.toUnsignedLong(auth.session().limits().getControlFrameBytes()))) throw new AuthFailure(AuthFailure.Code.INVALID);
                if(!channel.send(encoded)) throw new AuthFailure(AuthFailure.Code.STATE);
                byte[] response=receive(channel,deadline);
                ControlEnvelope result;
                try {result=nf.wire.ControlDecoder.decodeControl(response,auth.session().limits());}
                catch(nf.wire.WireFailure failure) {
                    if(failure.code()==nf.wire.WireFailure.Code.UNSUPPORTED) throw new AssertionError("CHAT_ENQUEUE_JAVA_FIRST expected Pending actual Unsupported after mutual auth");
                    throw failure;
                }
                if(!fence.accepts(context.runtimeSession()) || result.getRuntimeSession().getValue()!=context.runtimeSession()
                        || !result.hasChatEnqueueResult() || !result.getChatEnqueueResult().hasStatus()
                        || !result.getRequired().getCapabilityIdsList().equals(java.util.List.of(4))
                        || !result.getRequired().getSchemaIdsList().equals(java.util.List.of(4))) {
                    throw new AssertionError("CHAT_ENQUEUE_JAVA_FIRST expected correlated Pending result");
                }
                ChatOutgoingStatus status=result.getChatEnqueueResult().getStatus();
                nf.wire.ChatStatusAdmission.correlate(status,QueryChatOutgoing.newBuilder()
                        .setRequestId(command.getRequestId()).setPrincipal(command.getPrincipal())
                        .setUniverseId(command.getUniverseId()).setHistoryId(command.getHistoryId())
                        .setOriginalRequestId(command.getOriginalRequestId()).setMessageId(status.getMessageId()).build());
                if(status.getPhase()!=ChatOutgoingPhase.CHAT_OUTGOING_PHASE_PENDING || status.getOutboxRevision()!=1
                        || status.getSourceSequence()!=1 || !status.getReceiverReceipt().isEmpty()) {
                    throw new AssertionError("CHAT_ENQUEUE_JAVA_FIRST expected Pending/source1/revision1/noReceipt");
                }
                StringBuilder signed=new StringBuilder();
                for(byte value:status.getSignedMessage().toByteArray())signed.append(String.format("%02x",value&255));
                System.out.println("CHAT_ENQUEUE_SOCKET_OK "+signed);
                fence.invalidate();
                if(fence.accepts(runtime)) throw new AssertionError("CHAT_ENQUEUE_FENCE_INVALIDATION_FAILED");
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
