package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import com.google.protobuf.UnknownFieldSet;
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
import nf.wire.WireFailure;
import org.nearfuture.ipc.v1.LocalEndpointRole;
import org.nearfuture.protocol.v1.*;

/** Owned adversarial consumer fixtures, not forged server authorization or remote Ed25519 authority. */
public final class ChatStatusControlSocketClient {
    private ChatStatusControlSocketClient() { }
    private static final byte[] QUERY=repeated(111),ORIGINAL=repeated(101),MESSAGE=repeated(81);
    public static void main(String[] args) {
        try {run(args);System.out.println("CHAT_STATUS_CONTROLS_OK");}
        catch(AssertionError failure) {System.err.println(failure.getMessage());System.exit(1);}
        catch(Exception failure) {System.err.println("CHAT_CONTROLS_SETUP_OR_ADMISSION_FAILED:"+failure.getClass().getSimpleName());System.exit(2);}
    }
    private static void run(String[] args) throws Exception {
        if(args.length!=5) throw new AuthFailure(AuthFailure.Code.INVALID);
        String mode=args[1];int phase=Integer.parseInt(args[2]);
        byte[] expected=hex(args[3],174,2221),receipt=args[4].equals("-")?new byte[0]:hex(args[4],323,323);
        if(phase<1||phase>2||(phase==1&&receipt.length!=0)||(phase==2&&receipt.length!=323)) throw new AuthFailure(AuthFailure.Code.INVALID);
        try(Descriptor descriptor=Descriptor.read(Path.of(args[0]));Link baseline=Link.open(descriptor,descriptor.context.localLimits())) {
            check(baseline.auth.isActive(),"ACTUAL_MUTUAL_AUTH_REQUIRED");
            ChatOutgoingClient consumer=baseline.consumer();
            check(baseline.channel.send(consumer.query(QUERY,ORIGINAL,MESSAGE)),"ACTUAL_QUERY_QUEUE");
            byte[] bytes=baseline.receive();
            ChatOutgoingStatus actual=consumer.accept(bytes).status();
            check(actual.getPhaseValue()==phase&&actual.getOutboxRevision()==phase&&actual.getSourceSequence()==1
                &&Arrays.equals(actual.getSignedMessage().toByteArray(),expected)
                &&Arrays.equals(actual.getReceiverReceipt().toByteArray(),receipt),"INDEPENDENT_GENUINE_BASELINE");
            ControlEnvelope base=ControlEnvelope.parseFrom(bytes);
            if(mode.equals("refusals")) {
                profiles(baseline,base);shape(baseline,base);correlation(baseline,base);lifecycle(baseline,base);
                System.out.println("PASS_CLOSED_PROFILE_CANONICAL_CORRELATION_LIFECYCLE_PHASE_"+phase);
            } else {
                check(phase==1,"BUDGET_PENDING_BASELINE_REQUIRED");
                budget(descriptor,base,mode);
            }
        }
    }
    private static void profiles(Link link,ControlEnvelope base) {
        for(int profile:new int[]{1,2}) rejects(link,base.toBuilder().setRequired(required(profile)).build());
        rejects(link,base.toBuilder().setRequired(RequiredSemantics.newBuilder().addCapabilityIds(3).addCapabilityIds(1).addSchemaIds(3)).build());
        rejects(link,base.toBuilder().setRequired(required(3).toBuilder().clearSchemaIds()).build());
        ChatOutgoingStatus s=base.getChatOutgoingStatus();
        QueryOperation financial=QueryOperation.newBuilder().setRequestId(s.getRequestId()).setPrincipal(s.getPrincipal())
            .setUniverseId(s.getUniverseId()).setHistoryId(s.getHistoryId()).build();
        rejects(link,base.toBuilder().setQueryOperation(financial).setRequired(required(3)).build());
        byte[] unknown=concat(base.toByteArray(),new byte[]{(byte)0xf8,0x7f,1});
        rejects(link,unknown,WireFailure.Code.UNKNOWN_FIELD);
        rejects(link,concat(base.toByteArray(),new byte[]{0x08,1}),WireFailure.Code.DUPLICATE);
        UnknownFieldSet extra=UnknownFieldSet.newBuilder().addField(77,UnknownFieldSet.Field.newBuilder().addVarint(1).build()).build();
        rejects(link,base.toBuilder().setRequired(required(3).toBuilder().setUnknownFields(extra)).build());
    }
    private static void shape(Link link,ControlEnvelope base) {
        ChatOutgoingStatus s=base.getChatOutgoingStatus();
        rejects(link,with(base,s.toBuilder().setRequestId(RequestId.newBuilder().setValue(ByteString.copyFrom(new byte[16]))).build()));
        rejects(link,with(base,s.toBuilder().setMessageId(ChatMessageId.newBuilder().setValue(ByteString.copyFrom(new byte[15]))).build()));
        for(long revision:new long[]{0,8193}) rejects(link,with(base,s.toBuilder().setOutboxRevision(revision).build()));
        for(int phase:new int[]{0,3}) rejects(link,with(base,s.toBuilder().setPhaseValue(phase).build()));
        rejects(link,with(base,s.toBuilder().setSourceSequence(0).build()));
        for(int length:new int[]{0,173,2222}) rejects(link,with(base,s.toBuilder().setSignedMessage(ByteString.copyFrom(new byte[length])).build()));
        byte[] original=s.getSignedMessage().toByteArray();
        for(int at:new int[]{0,18,51,83,99,107,109}) {
            byte[] malformed=original.clone();malformed[at]^=(byte)(at==109?0xff:1);
            rejects(link,with(base,s.toBuilder().setSignedMessage(ByteString.copyFrom(malformed)).build()));
        }
        rejects(link,with(base,s.toBuilder().setSignedMessage(ByteString.copyFrom(concat(original,new byte[]{1}))).build()));
        if(s.getPhaseValue()==1) rejects(link,with(base,s.toBuilder().setReceiverReceipt(ByteString.copyFrom(new byte[]{1})).build()));
        else for(int length:new int[]{0,322,324}) rejects(link,with(base,s.toBuilder().setReceiverReceipt(ByteString.copyFrom(new byte[length])).build()));
    }
    private static void correlation(Link link,ControlEnvelope base) {
        ChatOutgoingStatus s=base.getChatOutgoingStatus();
        rejects(link,base.toBuilder().setRuntimeSession(RuntimeSession.newBuilder().setValue(link.auth.session().context().runtimeSession()+1)).build());
        rejects(link,with(base,s.toBuilder().setRequestId(RequestId.newBuilder().setValue(ByteString.copyFrom(repeated(112)))).build()));
        rejects(link,with(base,s.toBuilder().setOriginalRequestId(RequestId.newBuilder().setValue(ByteString.copyFrom(repeated(102)))).build()));
        rejects(link,with(base,s.toBuilder().setUniverseId(UniverseId.newBuilder().setValue(ByteString.copyFrom(repeated(9)))).build()));
        rejects(link,with(base,s.toBuilder().setHistoryId(HistoryId.newBuilder().setValue(ByteString.copyFrom(repeated(9)))).build()));
        rejects(link,with(base,s.toBuilder().setPrincipal(s.getPrincipal().toBuilder().setDeviceId(DeviceId.newBuilder().setValue(ByteString.copyFrom(repeated(9))))).build()));
        if(s.getPhaseValue()==2) {
            byte[] genuine=s.getReceiverReceipt().toByteArray();
            // Adversarial local input mutations, never an assertion of authenticated server corruption.
            for(int at:new int[]{0,18,50,66,82,147,163,179,195,211,227}) {
                byte[] bad=genuine.clone();bad[at]^=1;
                rejects(link,with(base,s.toBuilder().setReceiverReceipt(ByteString.copyFrom(bad)).build()));
            }
            byte[] zeroCursor=genuine.clone();Arrays.fill(zeroCursor,219,227,(byte)0);
            rejects(link,with(base,s.toBuilder().setReceiverReceipt(ByteString.copyFrom(zeroCursor)).build()));
        }
    }
    private static void lifecycle(Link link,ControlEnvelope base) {
        ChatOutgoingClient c=link.consumer();c.query(QUERY,ORIGINAL,MESSAGE);c.accept(base.toByteArray());
        expectState(()->c.accept(base.toByteArray()));
        SessionFence fence=new SessionFence(link.auth.session().context().runtimeSession());
        ChatOutgoingClient stale=new ChatOutgoingClient(link.auth.session(),fence);stale.query(QUERY,ORIGINAL,MESSAGE);fence.invalidate();
        expectState(()->stale.accept(base.toByteArray()));expectState(()->stale.query(QUERY,ORIGINAL,MESSAGE));
        SessionFence wrong=new SessionFence(link.auth.session().context().runtimeSession()+1);
        expectState(()->new ChatOutgoingClient(link.auth.session(),wrong));
    }
    private static void budget(Descriptor descriptor,ControlEnvelope base,String mode) throws Exception {
        ResourceLimits defaults=descriptor.context.localLimits();
        try(Link smallFrame=Link.open(descriptor,defaults.toBuilder().setControlFrameBytes(64).setChunkBytes(64).build())) {
            check(smallFrame.auth.session().limits().getControlFrameBytes()==64,"NEGOTIATED_CONTROL64");
            expectWire(()->smallFrame.consumer().query(QUERY,ORIGINAL,MESSAGE));
        }
        ResourceLimits tight=switch(mode) {
            case "depth" -> defaults.toBuilder().setNestingDepth(1).build();
            case "items" -> defaults.toBuilder().setCollectionItems(1).build();
            case "decoded" -> defaults.toBuilder().setControlFrameBytes(512).setChunkBytes(512).setDecodedBytes(512).build();
            default -> throw new AuthFailure(AuthFailure.Code.INVALID);
        };
        try(Link limited=Link.open(descriptor,tight)) {
            check(limited.auth.isActive()&&limited.auth.session().limits().equals(tight),"GENUINE_SELECTED_TIGHT_LIMITS");
            ChatOutgoingClient c=limited.consumer();c.query(QUERY,ORIGINAL,MESSAGE);
            byte[] adversarial=base.toByteArray();
            if(mode.equals("items")) adversarial=base.toBuilder().setRequired(required(3).toBuilder().addCapabilityIds(3)).build().toByteArray();
            if(mode.equals("decoded")) check(adversarial.length<512,"RAW_WITHIN512_DECODED_MESSAGE_BUDGET_INDEPENDENT");
            byte[] input=adversarial;
            // Real newly authenticated session. Replayed/mutated baseline is test-only input, not a fake ActiveAuth.
            expectLimit(()->c.accept(input),"NEGOTIATED_"+mode.toUpperCase()+"_MUST_REFUSE_BEFORE_PROJECTION");
        }
        System.out.println("PASS_NEGOTIATED_"+mode.toUpperCase());
    }
    private static void rejects(Link link,ControlEnvelope bad) {rejects(link,bad.toByteArray(),null);}
    private static void rejects(Link link,byte[] bytes,WireFailure.Code expected) {
        ChatOutgoingClient c=link.consumer();c.query(QUERY,ORIGINAL,MESSAGE);
        try {c.accept(bytes);throw new AssertionError("MALFORMED_OR_MISMATCHED_REPORT_MUST_REFUSE");}
        catch(WireFailure failure) {if(expected!=null) check(failure.code()==expected,"EXACT_CLOSED_WIRE_FAILURE");}
    }
    private static void expectWire(Runnable action) {try {action.run();throw new AssertionError("FRAME_LIMIT_MUST_REFUSE");}catch(WireFailure expected) { }}
    private static void expectLimit(Runnable action,String label) {
        try {action.run();throw new AssertionError(label);}
        catch(WireFailure failure) {check(failure.code()==WireFailure.Code.LIMIT,label+"_LIMIT_PRECEDES_SEMANTIC");}
    }
    private static void expectState(Runnable action) {try {action.run();throw new AssertionError("STALE_OR_DUPLICATE_MUST_REFUSE");}catch(AuthFailure failure) {check(failure.code()==AuthFailure.Code.STATE,"EXACT_STATE_REFUSAL");}}
    private static ControlEnvelope with(ControlEnvelope base,ChatOutgoingStatus value) {return base.toBuilder().setChatOutgoingStatus(value).build();}
    private static RequiredSemantics required(int profile) {return RequiredSemantics.newBuilder().addCapabilityIds(profile).addSchemaIds(profile).build();}
    private static byte[] repeated(int value) {byte[] b=new byte[16];Arrays.fill(b,(byte)value);return b;}
    private static byte[] concat(byte[] a,byte[] b) {byte[] result=Arrays.copyOf(a,a.length+b.length);System.arraycopy(b,0,result,a.length,b.length);return result;}
    private static void check(boolean value,String label) {if(!value) throw new AssertionError(label);}
    private static byte[] hex(String value,int min,int max) {
        if(value.length()%2!=0||value.length()<min*2||value.length()>max*2) throw new AuthFailure(AuthFailure.Code.INVALID);
        byte[] b=new byte[value.length()/2];for(int i=0;i<b.length;i++) {int hi=Character.digit(value.charAt(i*2),16),lo=Character.digit(value.charAt(i*2+1),16);if(hi<0||lo<0) throw new AuthFailure(AuthFailure.Code.INVALID);b[i]=(byte)(hi*16+lo);}return b;
    }
    private static final class Link implements AutoCloseable {
        final MutualAuthClient auth;final BackgroundChannel channel;final long deadline;
        private Link(MutualAuthClient auth,BackgroundChannel channel,long deadline) {this.auth=auth;this.channel=channel;this.deadline=deadline;}
        static Link open(Descriptor descriptor,ResourceLimits offered) throws Exception {
            MutualAuthClient auth=new MutualAuthClient(descriptor.context,descriptor.token,offered);
            BackgroundChannel channel;
            try {channel=new BackgroundChannel(new InetSocketAddress(InetAddress.getByAddress(new byte[]{127,0,0,1}),descriptor.context.port()),4096);}
            catch(Exception failure) {auth.close();throw failure;}
            Link link=new Link(auth,channel,System.nanoTime()+3_000_000_000L);
            try {check(channel.send(auth.hello().toByteArray()),"REAL_HELLO_QUEUE");check(channel.send(auth.respond(link.receive()).toByteArray()),"REAL_PROOF_QUEUE");auth.finish(link.receive());return link;}
            catch(Exception|AssertionError failure) {link.close();throw failure;}
        }
        ChatOutgoingClient consumer() {return new ChatOutgoingClient(auth.session(),new SessionFence(auth.session().context().runtimeSession()));}
        byte[] receive() throws Exception {while(System.nanoTime()-deadline<0) {byte[] bytes=channel.poll();if(bytes!=null) return bytes;Thread.sleep(1);}throw new AuthFailure(AuthFailure.Code.PROTOCOL);}
        @Override public void close() throws java.io.IOException {auth.close();channel.close();}
    }
    private static final class Descriptor implements AutoCloseable {
        final AuthContext context;final byte[] token;
        private Descriptor(AuthContext context,byte[] token) {this.context=context;this.token=token;}
        static Descriptor read(Path path) throws Exception {
            byte[] record;try(InputStream input=Files.newInputStream(path)) {record=input.readNBytes(265);}
            byte[] token=new byte[32];boolean owned=false;
            try {
                if(record.length!=264) throw new AuthFailure(AuthFailure.Code.INVALID);
                ByteBuffer buffer=ByteBuffer.wrap(record).order(ByteOrder.LITTLE_ENDIAN);magic(buffer,"NF-BLOB-1\0");if(buffer.getInt()!=218) throw new AuthFailure(AuthFailure.Code.INVALID);
                MessageDigest hash=MessageDigest.getInstance("SHA-256");hash.update(record,0,232);if(!MessageDigest.isEqual(hash.digest(),Arrays.copyOfRange(record,232,264))) throw new AuthFailure(AuthFailure.Code.INVALID);
                magic(buffer,"NF-IPC-R2\0");int port=Short.toUnsignedInt(buffer.getShort());buffer.getShort();long runtime=buffer.getLong();
                ByteString universe=bytes(buffer,16),history=bytes(buffer,16),ruleset=bytes(buffer,32),policy=bytes(buffer,32),account=bytes(buffer,16),device=bytes(buffer,16);buffer.get(token);
                ResourceLimits limits=ResourceLimits.newBuilder().setControlFrameBytes(buffer.getInt()).setChunkBytes(buffer.getInt()).setInflightBytes(buffer.getInt()).setInflightItems(buffer.getInt()).setDecodedBytes(buffer.getInt()).setCollectionItems(buffer.getInt()).setNestingDepth(buffer.getInt()).setTransferBytes(buffer.getLong()).build();
                Principal principal=Principal.newBuilder().setAccountId(AccountId.newBuilder().setValue(account)).setDeviceId(DeviceId.newBuilder().setValue(device)).build();
                Descriptor value=new Descriptor(new AuthContext(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL,port,runtime,universe,history,ruleset,policy,principal,limits),token);owned=true;return value;
            } finally {Arrays.fill(record,(byte)0);if(!owned) Arrays.fill(token,(byte)0);}
        }
        @Override public void close() {Arrays.fill(token,(byte)0);}
        private static ByteString bytes(ByteBuffer buffer,int count) {byte[] b=new byte[count];buffer.get(b);return ByteString.copyFrom(b);}
        private static void magic(ByteBuffer buffer,String expected) {byte[] b=new byte[expected.length()];buffer.get(b);if(!Arrays.equals(b,expected.getBytes(StandardCharsets.US_ASCII))) throw new AuthFailure(AuthFailure.Code.INVALID);}
    }
}
