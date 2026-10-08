package nf.adapter.profiling;

import com.fs.starfarer.api.EveryFrameScript;
import jdk.jfr.Category;
import jdk.jfr.Event;
import jdk.jfr.Label;
import jdk.jfr.Name;
import jdk.jfr.StackTrace;

/** Emits this observer's own callback span, never a whole frame or another script's work. */
public final class CampaignCallbackObserver implements EveryFrameScript {
    private final String traceId;
    private boolean closed;

    public CampaignCallbackObserver(String traceId) {
        if (traceId == null || !traceId.matches("[a-f0-9]{32}")) throw new IllegalArgumentException();
        this.traceId = traceId;
    }

    @Override
    public boolean isDone() {
        return closed;
    }

    @Override
    public boolean runWhilePaused() {
        return true;
    }

    @Override
    public void advance(float amount) {
        if (closed) return;
        CallbackEvent event = new CallbackEvent();
        if (!event.isEnabled()) return;
        event.begin();
        event.traceId = traceId;
        event.end();
        event.commit();
    }

    public void close() {
        closed = true;
    }

    @Name("nf.CampaignCallback")
    @Label("NearFuture observer callback")
    @Category("NearFuture")
    @StackTrace(false)
    private static final class CallbackEvent extends Event {
        public String traceId;
    }
}
