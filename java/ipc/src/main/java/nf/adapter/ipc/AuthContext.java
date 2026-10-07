package nf.adapter.ipc;
import com.google.protobuf.ByteString;
import org.nearfuture.ipc.v1.LocalEndpointRole;
import org.nearfuture.protocol.v1.Principal;
import org.nearfuture.protocol.v1.ResourceLimits;
/** Immutable binding supplied from the owner's private local rendezvous descriptor. */
public record AuthContext(LocalEndpointRole role,int port,long runtimeSession,ByteString universe,ByteString history,ByteString ruleset,ByteString content,Principal principal,ResourceLimits localLimits) {
    public AuthContext {
        if(role!=LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL&&role!=LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK||port<1||port>65535||runtimeSession==0) AuthLimits.fail();
        width(universe,16);width(history,16);width(ruleset,32);width(content,32);
        if(principal==null||!principal.hasAccountId()||!principal.hasDeviceId()||!principal.getUnknownFields().asMap().isEmpty()||!principal.getAccountId().getUnknownFields().asMap().isEmpty()||!principal.getDeviceId().getUnknownFields().asMap().isEmpty()) AuthLimits.fail();
        width(principal.getAccountId().getValue(),16);width(principal.getDeviceId().getValue(),16);AuthLimits.validate(localLimits);
    }
    private static void width(ByteString bytes,int width) {if(bytes==null||bytes.size()!=width) AuthLimits.fail();}
}