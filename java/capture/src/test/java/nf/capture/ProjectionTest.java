package nf.capture;
import java.util.List;
import static nf.capture.Values.*;
final class ProjectionTest {
    private ProjectionTest() { }
    static void run() {
        CaptureSmoke.SyntheticSource source=new CaptureSmoke.SyntheticSource();
        Snapshot desired=new Snapshot(source.stamp(),source.values);
        ShadowProjection projection=new ShadowProjection(source,Thread.currentThread(),1);
        if (!projection.begin(desired,1,ShadowProjection.Mode.SHADOW)) throw new AssertionError("stage starts");
        if (projection.advance(0)!=ShadowProjection.Result.PARTIAL || projection.checkpoint()!=null)
            throw new AssertionError("partial stage must remain invisible to save checkpoint");
        if (projection.advance(1)!=ShadowProjection.Result.PUBLISHED) throw new AssertionError("complete atomic shadow swap");
        ShadowProjection.View checkpoint=projection.checkpoint();
        if (checkpoint.sequence()!=1 || !checkpoint.snapshot().equals(desired)) throw new AssertionError("complete checkpoint");
        if (projection.begin(desired,1,ShadowProjection.Mode.SHADOW)) throw new AssertionError("same sequence exact replay is idempotent");
        Snapshot changed=new Snapshot(source.stamp(),List.of(new Faction(CaptureSmoke.id(10),0,Float.floatToRawIntBits(80)),source.values.get(1)));
        try {projection.begin(changed,1,ShadowProjection.Mode.SHADOW);throw new AssertionError("sequence conflict");}
        catch (CaptureFailure failure) {if (failure.code()!=CaptureFailure.Code.CONFLICT) throw failure;}
        projection.begin(changed,2,ShadowProjection.Mode.SHADOW); projection.advance(2);
        if (projection.checkpoint()!=checkpoint) throw new AssertionError("save sees preceding complete view");
        source.generation++;
        if (projection.advance(3)!=ShadowProjection.Result.STALE || projection.checkpoint()!=checkpoint)
            throw new AssertionError("drift aborts stage and retains previous complete checkpoint");
        try {projection.begin(new Snapshot(source.stamp(),source.values),3,ShadowProjection.Mode.CANONICAL);throw new AssertionError("native authority unavailable");}
        catch (CaptureFailure failure) {if (failure.code()!=CaptureFailure.Code.NATIVE_UNAVAILABLE) throw failure;}
    }
}
