package nf.capture;
import java.util.ArrayList;
import java.util.List;
import static nf.capture.Values.*;
final class ValuesBoundaryTest {
    private ValuesBoundaryTest() { }
    static void run() {
        byte[] bytes=new byte[16];bytes[0]=1;Id stable=new Id(bytes);bytes[0]=2;
        byte[] copy=stable.bytes();copy[0]=3;
        if (stable.bytes()[0]!=1) throw new AssertionError("opaque ID owns bounded copied bytes");
        CaptureSmoke.SyntheticSource source=new CaptureSmoke.SyntheticSource();
        Snapshot snapshot=new Snapshot(source.stamp(),source.values);
        try {snapshot.inputs().clear();throw new AssertionError("immutable worker handoff");} catch (UnsupportedOperationException expected) { }
        if (snapshot.eligibility()!=Eligibility.SHADOW_ONLY) throw new AssertionError("cooperative capture cannot certify authority");
        LifecycleTest.expect(CaptureFailure.Code.INVALID,()->new Id(new byte[15]));
        LifecycleTest.expect(CaptureFailure.Code.INVALID,()->new Hash(new byte[33]));
        LifecycleTest.expect(CaptureFailure.Code.INVALID,()->new Faction(CaptureSmoke.id(10),0,Float.floatToRawIntBits(Float.NaN)));
        LifecycleTest.expect(CaptureFailure.Code.INVALID,()->new Relation(CaptureSmoke.id(12),0,CaptureSmoke.id(10),CaptureSmoke.id(11),Float.floatToRawIntBits(1.1f)));
        LifecycleTest.expect(CaptureFailure.Code.INVALID,()->new Config(CaptureSmoke.id(11),0,0));
        LifecycleTest.expect(CaptureFailure.Code.INVALID,()->new SourceStamp(source.context,0,List.of(new Revision(CaptureSmoke.id(10),0),new Revision(CaptureSmoke.id(10),1))));
        LifecycleTest.expect(CaptureFailure.Code.INVALID,()->new SourceStamp(source.context,0,List.of(new Revision(CaptureSmoke.id(128),0),new Revision(CaptureSmoke.id(127),0))));
        List<Revision> oversized=new ArrayList<>();for(int n=0;n<65;n++) oversized.add(new Revision(CaptureSmoke.id(n),0));
        LifecycleTest.expect(CaptureFailure.Code.LIMIT,()->new SourceStamp(source.context,0,oversized));
        LifecycleTest.expect(CaptureFailure.Code.LIMIT,()->new CampaignCapture(source,Thread.currentThread(),9));
        CaptureSmoke.SyntheticSource maximum=new CaptureSmoke.SyntheticSource();
        List<Input> inputs=new ArrayList<>();for(int n=1;n<=64;n++) inputs.add(new Faction(CaptureSmoke.id(n),0,Float.floatToRawIntBits(n)));
        maximum.values=List.copyOf(inputs);
        CampaignCapture capture=new CampaignCapture(maximum,Thread.currentThread(),8);capture.begin(0);
        for(int frame=0;frame<8;frame++) {
            CampaignCapture.Result result=capture.advance(frame);
            if (result!=(frame==7?CampaignCapture.Result.PUBLISHED:CampaignCapture.Result.PARTIAL)) throw new AssertionError("hard-cap capture per-frame work");
            if (maximum.reads!=(frame+1)*8) throw new AssertionError("exact bounded port reads");
        }
        if (capture.published().inputs().size()!=64) throw new AssertionError("hard-cap immutable collection");
        ShadowProjection projection=new ShadowProjection(source,Thread.currentThread(),1);
        projection.restore(new ShadowProjection.View(-1,snapshot));
        if (projection.begin(snapshot,-1,ShadowProjection.Mode.SHADOW)) throw new AssertionError("u64 sequence replay at upper boundary");
        LifecycleTest.expect(CaptureFailure.Code.CONFLICT,()->projection.begin(snapshot,1,ShadowProjection.Mode.SHADOW));
        LifecycleTest.expect(CaptureFailure.Code.NATIVE_UNAVAILABLE,()->projection.begin(snapshot,-2,ShadowProjection.Mode.OFFLOAD));
        projection.invalidate(); LifecycleTest.expect(CaptureFailure.Code.INACTIVE,projection::checkpoint);
    }
}
