package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import nf.wire.WireFailure;
import org.nearfuture.ipc.v1.*;
import org.nearfuture.protocol.v1.*;

/** Real transcript activation; these pure caller controls do not claim sockets or device signature authority. */
public final class ChatEnqueueClientTest {
    private ChatEnqueueClientTest() { }
    private static final String TEXT="Public API original";
    private static final byte[] REQUEST=id(111),ORIGINAL=id(101),MESSAGE=id(81);
    public static void main(String[] args) {
        String[] groups={"inputs","outstanding","fence","reply","depth","decoded","collection"};
        if(args.length>1) throw new AssertionError("one selected group at most");
        boolean matched=args.length==0;
        for(String group:groups) if(args.length==0 || args[0].equals(group)) {
            matched=true;
            switch(group) {
                case "inputs" -> inputs();case "outstanding" -> outstanding();case "fence" -> fence();
                case "reply" -> reply();case "depth" -> depth();case "decoded" -> decoded();case "collection" -> collection();
                default -> throw new AssertionError();
            }
            System.out.println("PASS: public Java enqueue "+group);
        }
        check(matched,"unknown group");
    }
    private static void inputs() {
        try(Active active=active(defaults(),1)) {
            SessionFence fence=new SessionFence(9);
            wire(WireFailure.Code.SEMANTIC,()->new ChatEnqueueClient(null,fence),"null session");
            wire(WireFailure.Code.SEMANTIC,()->new ChatEnqueueClient(active.auth.session(),null),"null fence");
            ChatEnqueueClient client=new ChatEnqueueClient(active.auth.session(),fence);
            wire(WireFailure.Code.SEMANTIC,()->client.enqueue(null,ORIGINAL,TEXT),"null request");
            wire(WireFailure.Code.SEMANTIC,()->client.enqueue(new byte[16],ORIGINAL,TEXT),"zero request");
            wire(WireFailure.Code.SEMANTIC,()->client.enqueue(REQUEST,new byte[15],TEXT),"short original");
            wire(WireFailure.Code.SEMANTIC,()->client.enqueue(REQUEST,new byte[16],TEXT),"zero original");
            wire(WireFailure.Code.SEMANTIC,()->client.enqueue(REQUEST,null,TEXT),"null original");
            wire(WireFailure.Code.SEMANTIC,()->client.enqueue(REQUEST,ORIGINAL,null),"null text");
            wire(WireFailure.Code.SEMANTIC,()->client.enqueue(REQUEST,ORIGINAL,""),"empty text");
            byte[] command=client.enqueue(REQUEST,ORIGINAL,TEXT);check(command.length>0,"invalid input cannot consume outstanding slot");
        }
        try(Active bulk=active(defaults(),2)) {
            wire(WireFailure.Code.SEMANTIC,()->new ChatEnqueueClient(bulk.auth.session(),new SessionFence(9)),"BULK not CONTROL");
        }
    }
    private static void outstanding() {
        try(Active active=active(defaults(),1)) {
            ChatEnqueueClient client=client(active);
            auth(()->client.accept(response(active,status(active,TEXT)).toByteArray()),"accept before command");
            client.enqueue(REQUEST,ORIGINAL,TEXT);
            auth(()->client.enqueue(id(112),ORIGINAL,TEXT),"one outstanding");
            ChatOutgoingValue value=client.accept(response(active,status(active,TEXT)).toByteArray());
            check(value.status().getPhase()==ChatOutgoingPhase.CHAT_OUTGOING_PHASE_PENDING,"immutable Pending value");
            check(value.status().getOutboxRevision()==1,"revision1");
            auth(()->client.accept(response(active,status(active,TEXT)).toByteArray()),"duplicate completion");
            check(client.enqueue(id(112),ORIGINAL,TEXT).length>0,"slot opens after valid completion");
        }
    }
    private static void fence() {
        try(Active active=active(defaults(),1)) {
            auth(()->new ChatEnqueueClient(active.auth.session(),new SessionFence(10)),"wrong current runtime");
            SessionFence before=new SessionFence(9);ChatEnqueueClient a=new ChatEnqueueClient(active.auth.session(),before);before.invalidate();
            auth(()->a.enqueue(REQUEST,ORIGINAL,TEXT),"stale before send");
            SessionFence after=new SessionFence(9);ChatEnqueueClient b=new ChatEnqueueClient(active.auth.session(),after);b.enqueue(REQUEST,ORIGINAL,TEXT);after.invalidate();
            auth(()->b.accept(response(active,status(active,TEXT)).toByteArray()),"stale before completion");
        }
    }
    private static void reply() {
        try(Active active=active(defaults(),1)) {
            ChatEnqueueClient client=client(active);client.enqueue(REQUEST,ORIGINAL,TEXT);ChatOutgoingStatus good=status(active,TEXT);
            wire(WireFailure.Code.SEMANTIC,()->client.accept(response(active,good.toBuilder().setRequestId(request(id(112))).build()).toByteArray()),"request correlation");
            wire(WireFailure.Code.SEMANTIC,()->client.accept(response(active,good.toBuilder().setOriginalRequestId(request(id(102))).build()).toByteArray()),"original correlation");
            Principal wrong=good.getPrincipal().toBuilder().setAccountId(AccountId.newBuilder().setValue(ByteString.copyFrom(id(7)))).build();
            wire(WireFailure.Code.SEMANTIC,()->client.accept(response(active,good.toBuilder().setPrincipal(wrong).build()).toByteArray()),"principal correlation");
            wire(WireFailure.Code.SEMANTIC,()->client.accept(response(active,good.toBuilder().setUniverseId(UniverseId.newBuilder().setValue(ByteString.copyFrom(id(7)))).build()).toByteArray()),"universe correlation");
            wire(WireFailure.Code.SEMANTIC,()->client.accept(response(active,good.toBuilder().setHistoryId(HistoryId.newBuilder().setValue(ByteString.copyFrom(id(7)))).build()).toByteArray()),"history correlation");
            wire(WireFailure.Code.SEMANTIC,()->client.accept(response(active,status(active,"Different original")).toByteArray()),"canonical text correlation");
            wire(WireFailure.Code.SEMANTIC,()->client.accept(response(active,good).toBuilder().setRuntimeSession(RuntimeSession.newBuilder().setValue(10)).build().toByteArray()),"runtime correlation");
            for(int profile:new int[]{1,2,3}) {
                byte[] wrongProfile=response(active,good).toBuilder().setRequired(RequiredSemantics.newBuilder().addCapabilityIds(profile).addSchemaIds(profile)).build().toByteArray();
                wire(WireFailure.Code.UNSUPPORTED,()->client.accept(wrongProfile),"profile isolation "+profile);
            }
            wire(WireFailure.Code.SEMANTIC,()->client.accept(response(active,good.toBuilder().setOutboxRevision(8193).build()).toByteArray()),"bounded revision");
            wire(WireFailure.Code.SEMANTIC,()->client.accept(response(active,good).toBuilder().setChatEnqueueResult(ChatEnqueueResult.getDefaultInstance()).build().toByteArray()),"missing status");
            wire(WireFailure.Code.SEMANTIC,()->client.accept(null),"null response");
            check(client.accept(response(active,good).toByteArray()).status().equals(good),"invalid replies never complete outstanding original");
        }
    }
    private static void depth() {
        ResourceLimits tight=defaults().toBuilder().setNestingDepth(1).build();
        try(Active active=active(tight,1)) {
            check(active.auth.session().limits().getNestingDepth()==1,"genuine negotiated depth1");
            wire(WireFailure.Code.LIMIT,()->client(active).enqueue(REQUEST,ORIGINAL,TEXT),"OUTGOING_DEPTH expected LIMIT before outstanding admission");
        }
    }
    private static void decoded() {
        ResourceLimits tight=defaults().toBuilder().setControlFrameBytes(256).setChunkBytes(128).setDecodedBytes(512).build();
        try(Active baseline=active(defaults(),1);Active active=active(tight,1)) {
            byte[] command=client(baseline).enqueue(REQUEST,ORIGINAL,TEXT);
            check(command.length<=256 && command.length<512,"raw command fits both limits; this is descriptor charge, not frame failure");
            check(active.auth.session().limits().getDecodedBytes()==512,"genuine negotiated decoded512");
            wire(WireFailure.Code.LIMIT,()->client(active).enqueue(REQUEST,ORIGINAL,TEXT),"OUTGOING_DECODED expected LIMIT for charged nested structure");
        }
    }
    private static void collection() {
        ResourceLimits tight=defaults().toBuilder().setCollectionItems(1).build();
        try(Active active=active(tight,1)) {
            byte[] encoded=client(active).enqueue(REQUEST,ORIGINAL,TEXT);
            try {
                ControlEnvelope command=ControlEnvelope.parseFrom(encoded);
                check(command.getRequired().getCapabilityIdsList().equals(java.util.List.of(4)) && command.getRequired().getSchemaIdsList().equals(java.util.List.of(4)),"one item per closed list fits minimum1");
            } catch(com.google.protobuf.InvalidProtocolBufferException failure) {throw new AssertionError(failure);}
        }
    }
    private static ChatEnqueueClient client(Active a) {return new ChatEnqueueClient(a.auth.session(),new SessionFence(9));}
    private static ResourceLimits defaults() {return AuthClientTest.limits();}
    private static final class Active implements AutoCloseable {
        final MutualAuthClient auth;
        Active(MutualAuthClient auth) {this.auth=auth;}
        public void close() {auth.close();}
    }
    private static Active active(ResourceLimits selected,int role) {
        AuthContext context=AuthClientTest.context(role,role==1?12345:12346,9);
        byte[] key=id(19);key=Arrays.copyOf(key,32);byte[] server=new byte[32];Arrays.fill(server,(byte)9);
        MutualAuthClient auth=new MutualAuthClient(context,key,selected);
        try {
            byte[] nonce=auth.hello().getHello().getClientNonce().toByteArray();
            byte[] proof=AuthTranscript.proof(key,AuthTranscript.encode(context,selected,selected,nonce,server,1));
            auth.respond(AuthClientTest.header(9).setChallenge(LocalAuthChallenge.newBuilder().setServerNonce(ByteString.copyFrom(server)).setSelectedLimits(selected).setServerProof(ByteString.copyFrom(proof))).build().toByteArray());
            byte[] finished=AuthTranscript.proof(key,AuthTranscript.encode(context,selected,selected,nonce,server,3));
            auth.finish(AuthClientTest.header(9).setAccepted(LocalAuthAccepted.newBuilder().setServerFinishedProof(ByteString.copyFrom(finished))).build().toByteArray());
            check(auth.isActive() && auth.session().limits().equals(selected),"actual mutual transcript activation");return new Active(auth);
        } catch(RuntimeException|Error failure) {auth.close();throw failure;}
        finally {Arrays.fill(key,(byte)0);}
    }
    private static ChatOutgoingStatus status(Active a,String text) {
        var c=a.auth.session().context();byte[] t=text.getBytes(StandardCharsets.UTF_8);
        byte[] signed=ByteBuffer.allocate(109+t.length+64).order(ByteOrder.BIG_ENDIAN)
                .put("NF-CHAT-MESSAGE-1\0".getBytes(StandardCharsets.US_ASCII))
                .put(c.universe().toByteArray()).put(c.history().toByteArray()).put((byte)1)
                .put(c.principal().getAccountId().getValue().toByteArray()).put(c.principal().getDeviceId().getValue().toByteArray())
                .put(MESSAGE).putLong(1).putShort((short)t.length).put(t).put(new byte[64]).array();
        return ChatOutgoingStatus.newBuilder().setRequestId(request(REQUEST)).setPrincipal(c.principal())
                .setUniverseId(UniverseId.newBuilder().setValue(c.universe())).setHistoryId(HistoryId.newBuilder().setValue(c.history()))
                .setOriginalRequestId(request(ORIGINAL)).setMessageId(ChatMessageId.newBuilder().setValue(ByteString.copyFrom(MESSAGE)))
                .setPhase(ChatOutgoingPhase.CHAT_OUTGOING_PHASE_PENDING).setSourceSequence(1).setOutboxRevision(1).setSignedMessage(ByteString.copyFrom(signed)).build();
    }
    private static ControlEnvelope response(Active a,ChatOutgoingStatus s) {return ControlEnvelope.newBuilder().setProtocolVersion(1).setRuntimeSession(RuntimeSession.newBuilder().setValue(a.auth.session().context().runtimeSession())).setRequired(RequiredSemantics.newBuilder().addCapabilityIds(4).addSchemaIds(4)).setChatEnqueueResult(ChatEnqueueResult.newBuilder().setStatus(s)).build();}
    private static RequestId request(byte[] bytes) {return RequestId.newBuilder().setValue(ByteString.copyFrom(bytes)).build();}
    private static byte[] id(int b) {byte[] bytes=new byte[16];Arrays.fill(bytes,(byte)b);return bytes;}
    private static void check(boolean condition,String label) {if(!condition) throw new AssertionError(label);}
    private static void wire(WireFailure.Code code,Runnable f,String label) {try {f.run();throw new AssertionError(label+": expected "+code);} catch(WireFailure failure) {check(failure.code()==code,label+": actual "+failure.code());}}
    private static void auth(Runnable f,String label) {try {f.run();throw new AssertionError(label+": expected STATE");} catch(AuthFailure failure) {check(failure.code()==AuthFailure.Code.STATE,label+": actual "+failure.code());}}
}
