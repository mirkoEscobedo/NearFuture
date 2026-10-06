package nf.capture;
import java.util.ArrayList;
import static nf.capture.Values.*;
/** Shadow values only. Native mutation modes remain fail-closed until actual reviewed G1/G2 seams exist. */
public final class ShadowProjection {
    public enum Mode { SHADOW, OFFLOAD, CANONICAL }
    public enum Result { IDLE, PARTIAL, PUBLISHED, STALE }
    public record View(long sequence,Snapshot snapshot) {
        public View {if (sequence==0||snapshot==null) Values.fail();}
    }
    private final CaptureSource source;
    private final CampaignBudget budget;
    private final ArrayList<Input> staged=new ArrayList<>();
    private View current;
    private View pending;
    private boolean active=true;
    public ShadowProjection(CaptureSource source,Thread owner,int perFrame) {
        if (source==null) Values.fail();this.source=source;this.budget=new CampaignBudget(owner,perFrame);
    }
    public boolean begin(Snapshot desired,long sequence,Mode mode) {
        budget.thread();ensureActive(); if (desired==null||mode==null||sequence==0) Values.fail();
        if (mode!=Mode.SHADOW) throw new CaptureFailure(CaptureFailure.Code.NATIVE_UNAVAILABLE);
        if (!desired.certificate().equals(stamp())) throw new CaptureFailure(CaptureFailure.Code.DRIFT);
        View candidate=new View(sequence,desired);
        if (current!=null && Long.compareUnsigned(sequence,current.sequence())<=0) {
            if (candidate.equals(current)) return false;
            throw new CaptureFailure(CaptureFailure.Code.CONFLICT);
        }
        if (pending!=null && Long.compareUnsigned(sequence,pending.sequence())<=0 && !candidate.equals(pending))
            throw new CaptureFailure(CaptureFailure.Code.CONFLICT);
        pending=candidate;staged.clear();return true;
    }
    public Result advance(long frame) {
        budget.frame(frame);ensureActive(); if (pending==null) return Result.IDLE;
        if (!pending.snapshot().certificate().equals(stamp())) return stale();
        while (staged.size()<pending.snapshot().inputs().size() && budget.take())
            staged.add(pending.snapshot().inputs().get(staged.size()));
        if (!pending.snapshot().certificate().equals(stamp())) return stale();
        if (staged.size()!=pending.snapshot().inputs().size()) return Result.PARTIAL;
        View complete=new View(pending.sequence(),new Snapshot(pending.snapshot().certificate(),staged));
        current=complete;pending=null;staged.clear();return Result.PUBLISHED;
    }
    public View checkpoint() {
        budget.thread();ensureActive();
        if (current!=null && !current.snapshot().certificate().context().equals(stamp().context())) { current=null;stale(); }
        return current;
    }
    public void restore(View checkpoint) {
        budget.thread();ensureActive();if (checkpoint==null) Values.fail();
        if (current!=null||pending!=null) throw new CaptureFailure(CaptureFailure.Code.CONFLICT);
        if (!checkpoint.snapshot().certificate().equals(stamp())) throw new CaptureFailure(CaptureFailure.Code.DRIFT);
        current=checkpoint;
    }
    private SourceStamp stamp() {
        try {SourceStamp value=source.stamp();if (value==null) Values.fail();return value;}
        catch (RuntimeException ignored) {current=null;stale();throw new CaptureFailure(CaptureFailure.Code.SOURCE);}
    }
    private Result stale() {pending=null;staged.clear();return Result.STALE;}
    private void ensureActive() {if (!active) throw new CaptureFailure(CaptureFailure.Code.INACTIVE);}
    public void invalidate() {budget.thread();active=false;current=null;stale();}
}
