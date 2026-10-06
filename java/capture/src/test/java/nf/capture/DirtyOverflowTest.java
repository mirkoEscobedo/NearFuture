package nf.capture;
import java.util.List;
import static nf.capture.Values.*;
final class DirtyOverflowTest {
    private DirtyOverflowTest() { }
    static void run() {
        CaptureSmoke.SyntheticSource source=new CaptureSmoke.SyntheticSource();
        for (boolean generationOverflow:List.of(false,true)) {
            SourceStamp initial=new SourceStamp(source.context,generationOverflow?Long.MAX_VALUE:0,
                List.of(new Revision(CaptureSmoke.id(10),generationOverflow?0:Long.MAX_VALUE)));
            ObservedRevisions dirty=new ObservedRevisions(Thread.currentThread(),initial);
            SourceStamp old=dirty.stamp();
            LifecycleTest.expect(CaptureFailure.Code.LIMIT,()->dirty.observe(CaptureSmoke.id(10)));
            LifecycleTest.expect(CaptureFailure.Code.INACTIVE,dirty::stamp);
            LifecycleTest.expect(CaptureFailure.Code.INACTIVE,dirty::dirty);
            LifecycleTest.expect(CaptureFailure.Code.INACTIVE,()->dirty.acknowledge(old));
            LifecycleTest.expect(CaptureFailure.Code.INACTIVE,()->dirty.replace(source.context,List.of(new Revision(CaptureSmoke.id(10),0))));
            Context next=new Context(source.context.universe(),source.context.history(),source.context.campaign(),source.context.branch(),source.context.provider(),2,source.context.content(),source.context.ruleset(),1);
            ObservedRevisions fresh=new ObservedRevisions(Thread.currentThread(),new SourceStamp(next,0,List.of(new Revision(CaptureSmoke.id(10),0))));
            LifecycleTest.expect(CaptureFailure.Code.DRIFT,()->fresh.acknowledge(old));
            fresh.observe(CaptureSmoke.id(10));
            if (fresh.stamp().generation()!=1||fresh.stamp().readSet().get(0).revision()!=1) throw new AssertionError("explicit new-session owner can observe fresh revisions");
        }
    }
}
