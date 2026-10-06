package nf.capture;
/** Explicit frame input; the adapter must supply actual monotonically increasing campaign frames. */
final class CampaignBudget {
    private final Thread owner;
    private final int limit;
    private long frame=-1;
    private int remaining;
    CampaignBudget(Thread owner,int limit) {
        if (owner==null||limit<1||limit>Values.MAX_PER_FRAME) throw new CaptureFailure(CaptureFailure.Code.LIMIT);
        this.owner=owner;this.limit=limit;
    }
    void thread() {if (Thread.currentThread()!=owner) throw new CaptureFailure(CaptureFailure.Code.WRONG_THREAD);}
    void frame(long next) {
        thread(); if (next<0||next<frame) Values.fail();
        if (next>frame) {frame=next;remaining=limit;}
    }
    boolean take() {if (remaining==0) return false;remaining--;return true;}
}
