package nf.capture;
import static nf.capture.Values.*;
final class DirtyIndexTest {
    private DirtyIndexTest() { }
    static void run() {
        CaptureSmoke.SyntheticSource source=new CaptureSmoke.SyntheticSource();
        ObservedRevisions dirty=new ObservedRevisions(Thread.currentThread(),source.stamp());
        dirty.acknowledge(dirty.stamp());
        SourceStamp old=dirty.stamp();
        dirty.observe(CaptureSmoke.id(10)); dirty.observe(CaptureSmoke.id(10));
        if (dirty.dirty().size()!=1 || dirty.stamp().readSet().get(0).revision()!=2)
            throw new AssertionError("coalesced dirty aggregate and exact revision");
        try {dirty.acknowledge(old);throw new AssertionError("stale capture cannot clear dirtiness");}
        catch (CaptureFailure failure) {if (failure.code()!=CaptureFailure.Code.DRIFT) throw failure;}
        if (dirty.dirty().size()!=1) throw new AssertionError("stale acknowledgement retains pending mutation");
        dirty.acknowledge(dirty.stamp()); if (!dirty.dirty().isEmpty()) throw new AssertionError("exact captured frontier clears");
    }
}
