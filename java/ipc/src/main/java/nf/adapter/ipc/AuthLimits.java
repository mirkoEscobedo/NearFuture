package nf.adapter.ipc;
import org.nearfuture.protocol.v1.ResourceLimits;
final class AuthLimits {
    private AuthLimits() { }
    static long[] values(ResourceLimits limits) {return new long[]{Integer.toUnsignedLong(limits.getControlFrameBytes()),Integer.toUnsignedLong(limits.getChunkBytes()),Integer.toUnsignedLong(limits.getInflightBytes()),Integer.toUnsignedLong(limits.getInflightItems()),Integer.toUnsignedLong(limits.getDecodedBytes()),Integer.toUnsignedLong(limits.getCollectionItems()),Integer.toUnsignedLong(limits.getNestingDepth()),limits.getTransferBytes()};}
    static void validate(ResourceLimits limits) {
        if(limits==null||!limits.getUnknownFields().asMap().isEmpty()) fail();
        long[] values=values(limits),caps={1048576,262144,16777216,256,1048576,4096,32,67108864};
        for(int i=0;i<8;i++) if(values[i]<=0||values[i]>caps[i]) fail();
        if(values[1]>values[7]||values[1]>values[0]||values[0]>values[4]||values[0]>values[2]) fail();
    }
    static ResourceLimits minimum(ResourceLimits offered,ResourceLimits local) {
        validate(offered);validate(local);long[] a=values(offered),b=values(local);for(int i=0;i<8;i++) a[i]=Math.min(a[i],b[i]);
        ResourceLimits result=ResourceLimits.newBuilder().setControlFrameBytes((int)a[0]).setChunkBytes((int)a[1]).setInflightBytes((int)a[2]).setInflightItems((int)a[3]).setDecodedBytes((int)a[4]).setCollectionItems((int)a[5]).setNestingDepth((int)a[6]).setTransferBytes(a[7]).build();validate(result);return result;
    }
    static void fail() {throw new AuthFailure(AuthFailure.Code.INVALID);}
}