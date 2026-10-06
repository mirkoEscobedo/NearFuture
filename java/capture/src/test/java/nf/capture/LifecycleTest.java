package nf.capture;
import java.util.List;
import java.util.concurrent.atomic.AtomicReference;
import static nf.capture.Values.*;
final class LifecycleTest {
    private LifecycleTest() { }
    static void run() throws InterruptedException {
        CaptureSmoke.SyntheticSource initial=new CaptureSmoke.SyntheticSource(); Context before=initial.context;
        List<Context> replacements=List.of(
            new Context(CaptureSmoke.id(99),before.history(),before.campaign(),before.branch(),before.provider(),1,before.content(),before.ruleset(),1),
            new Context(before.universe(),CaptureSmoke.id(99),before.campaign(),before.branch(),before.provider(),1,before.content(),before.ruleset(),1),
            new Context(before.universe(),before.history(),CaptureSmoke.id(99),before.branch(),before.provider(),1,before.content(),before.ruleset(),1),
            new Context(before.universe(),before.history(),before.campaign(),CaptureSmoke.id(99),before.provider(),1,before.content(),before.ruleset(),1),
            new Context(before.universe(),before.history(),before.campaign(),before.branch(),CaptureSmoke.id(99),1,before.content(),before.ruleset(),1),
            new Context(before.universe(),before.history(),before.campaign(),before.branch(),before.provider(),2,before.content(),before.ruleset(),1),
            new Context(before.universe(),before.history(),before.campaign(),before.branch(),before.provider(),1,CaptureSmoke.hash(99),before.ruleset(),1),
            new Context(before.universe(),before.history(),before.campaign(),before.branch(),before.provider(),1,before.content(),CaptureSmoke.hash(99),1),
            new Context(before.universe(),before.history(),before.campaign(),before.branch(),before.provider(),1,before.content(),before.ruleset(),2));
        for (Context next:replacements) {
            CaptureSmoke.SyntheticSource source=new CaptureSmoke.SyntheticSource();
            CampaignCapture capture=new CampaignCapture(source,Thread.currentThread(),1); capture.begin(0);capture.advance(0);
            ShadowProjection projection=new ShadowProjection(source,Thread.currentThread(),1);
            projection.begin(new Snapshot(source.stamp(),source.values),1,ShadowProjection.Mode.SHADOW);projection.advance(0);
            source.context=next;
            if (capture.advance(1)!=CampaignCapture.Result.STALE || capture.published()!=null || projection.advance(1)!=ShadowProjection.Result.STALE || projection.checkpoint()!=null)
                throw new AssertionError("every context identity fences queued capture/projection");
        }
        CaptureSmoke.SyntheticSource source=new CaptureSmoke.SyntheticSource();
        CampaignCapture capture=new CampaignCapture(source,Thread.currentThread(),1);capture.begin(0);capture.advance(0);
        source.values=List.of(new Faction(CaptureSmoke.id(10),1,Float.floatToRawIntBits(76)),source.values.get(1));
        if (capture.advance(1)!=CampaignCapture.Result.STALE) throw new AssertionError("revision drift without generation still rejects");
        capture.invalidate();
        expect(CaptureFailure.Code.INACTIVE,()->capture.begin(2));
        AtomicReference<CaptureFailure.Code> failure=new AtomicReference<>();
        Thread worker=new Thread(()->{try {capture.begin(3);} catch (CaptureFailure caught) {failure.set(caught.code());}});
        worker.start();worker.join(1000);
        if (worker.isAlive()) {worker.interrupt();worker.join(1000);throw new AssertionError("bounded owned value-only worker");}
        if (failure.get()!=CaptureFailure.Code.WRONG_THREAD) throw new AssertionError("campaign port unavailable to worker");
    }
    static void expect(CaptureFailure.Code code,Runnable operation) {
        try {operation.run();throw new AssertionError("expected "+code);}
        catch (CaptureFailure failure) {if (failure.code()!=code) throw failure;}
    }
}
