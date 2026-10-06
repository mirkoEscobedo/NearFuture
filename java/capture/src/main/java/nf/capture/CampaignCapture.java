package nf.capture;
import java.util.ArrayList;
import static nf.capture.Values.*;
/** Bounded cooperative capture; missing native mutation hooks never confer authority eligibility. */
public final class CampaignCapture {
    public enum Result { IDLE, PARTIAL, PUBLISHED, STALE }
    private final CaptureSource source;
    private final CampaignBudget budget;
    private SourceStamp start;
    private final ArrayList<Input> staged=new ArrayList<>();
    private Snapshot published;
    private boolean active=true;
    private int staleCount;
    private long retryAt, currentFrame;
    public CampaignCapture(CaptureSource source,Thread owner,int perFrame) {
        if (source==null||owner==null||perFrame<1||perFrame>MAX_PER_FRAME) throw new CaptureFailure(CaptureFailure.Code.LIMIT);
        this.source=source;this.budget=new CampaignBudget(owner,perFrame);
    }
    private void thread() {budget.thread();}
    public boolean begin(long frame) {
        thread(); if (!active) throw new CaptureFailure(CaptureFailure.Code.INACTIVE);
        budget.frame(frame); currentFrame=frame; if (frame<retryAt) return false; start=stamp();staged.clear();published=null;return true;
    }
    public Result advance(long frame) {
        thread(); if (!active) throw new CaptureFailure(CaptureFailure.Code.INACTIVE);
        budget.frame(frame); currentFrame=frame; if (start==null) return Result.IDLE;
        if (!start.equals(stamp())) return stale();
        while (staged.size()<start.readSet().size() && budget.take()) {
            Revision expected=start.readSet().get(staged.size()); Input value=read(expected.aggregate());
            if (value==null||!value.aggregate().equals(expected.aggregate())||value.revision()!=expected.revision()) return stale();
            staged.add(value);
        }
        if (!start.equals(stamp())) return stale();
        if (staged.size()!=start.readSet().size()) return Result.PARTIAL;
        published=new Snapshot(start,staged); staleCount=0; start=null;staged.clear();return Result.PUBLISHED;
    }
    private SourceStamp stamp() {
        try {SourceStamp value=source.stamp();if (value==null) Values.fail();return value;}
        catch (RuntimeException ignored) {stale();throw new CaptureFailure(CaptureFailure.Code.SOURCE);}
    }
    private Input read(Id aggregate) {
        try {return source.read(aggregate);}
        catch (RuntimeException ignored) {stale();throw new CaptureFailure(CaptureFailure.Code.SOURCE);}
    }
    private Result stale() {start=null;staged.clear();published=null; if (++staleCount>=8) { retryAt=currentFrame>Long.MAX_VALUE-32?Long.MAX_VALUE:currentFrame+32; staleCount=0; } return Result.STALE;}
    public Snapshot published() {thread();return published;}
    public void invalidate() {thread();active=false;stale();}
}
