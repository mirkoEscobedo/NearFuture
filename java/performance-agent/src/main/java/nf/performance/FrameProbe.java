package nf.performance;

import java.util.concurrent.atomic.AtomicReference;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import jdk.jfr.Category;
import jdk.jfr.Event;
import jdk.jfr.FlightRecorder;
import jdk.jfr.FlightRecorderListener;
import jdk.jfr.Recording;
import jdk.jfr.RecordingState;
import jdk.jfr.Label;
import jdk.jfr.Name;
import jdk.jfr.StackTrace;

/** Bounded presentation marker: numeric values only, no game objects, I/O, or waits. */
public final class FrameProbe {
    private static final AtomicReference<Thread> OWNER = new AtomicReference<>();
    private static final AtomicInteger REJECTION = new AtomicInteger();
    private static final AtomicBoolean REJECTION_RECORDED = new AtomicBoolean();
    private static final AtomicReference<Recording> SESSION = new AtomicReference<>();
    private static volatile boolean enabled;
    private static volatile boolean recordingRunning;
    private static String traceId;
    private static String classSha256;
    private static boolean observerEnabled;
    private static boolean previous;
    private static long lastNanos;
    private static long frameId;
    private FrameProbe() { }

    static void configure(String trace, String selectedClass, boolean callbackObserver) {
        traceId = trace;
        classSha256 = selectedClass;
        observerEnabled = callbackObserver;
        FlightRecorder.register(Interval.class);
        FlightRecorder.register(Rejected.class);
        FlightRecorder.register(Configuration.class);
        enabled = true;
        FlightRecorder.addListener(new FlightRecorderListener() {
            @Override public void recorderInitialized(FlightRecorder recorder) {
                var recordings = recorder.getRecordings();
                if (recordings.size() > 64) { reject(5); return; }
                for (Recording recording : recordings) observeRecording(recording);
            }
            @Override public void recordingStateChanged(Recording recording) { observeRecording(recording); }
        });
    }
    private static void observeRecording(Recording recording) {
        if (!recording.getName().equals("NF100")) return;
        if (recording.getState() == RecordingState.RUNNING) {
            recordingRunning = true;
            if (!SESSION.compareAndSet(null, recording) && SESSION.get() != recording) { reject(5); return; }
            Configuration event = new Configuration();
            if (event.isEnabled()) {
                event.traceId = traceId; event.classSha256 = classSha256;
                event.observerEnabled = observerEnabled; event.commit();
            }
        } else if (SESSION.get() == recording && (recording.getState() == RecordingState.STOPPED
            || recording.getState() == RecordingState.CLOSED)) {
            // A normal end keeps the completed recording valid, but the trace can
            // never resume in another recording, even if no presentation occurred in between.
            REJECTION.compareAndSet(0, 4); enabled = false; recordingRunning = false;
        }
    }
    static void reject(int reason) {
        REJECTION.compareAndSet(0, reason);
        enabled = false;
        emitRejection();
    }
    private static void emitRejection() {
        if (REJECTION.get() == 0) return;
        Rejected event = new Rejected();
        if (event.isEnabled() && REJECTION_RECORDED.compareAndSet(false, true)) {
            event.reason = REJECTION.get(); event.commit();
        }
    }
    /** Called only after a successful pinned swapBuffers return; never treats callback duration as a frame. */
    public static void present() {
        long now = System.nanoTime();
        if (!enabled) { emitRejection(); return; }
        if (!recordingRunning) return;
        Thread thread = Thread.currentThread();
        OWNER.compareAndSet(null, thread);
        if (OWNER.get() != thread) { reject(2); return; }
        Interval event = new Interval();
        if (!event.isEnabled()) {
            if (previous) reject(4);
            previous = false; return;
        }
        event.begin();
        event.clockUncertaintyNanos = System.nanoTime() - now;
        if (event.clockUncertaintyNanos < 0) { reject(3); return; }
        if (previous) {
            long elapsed = now - lastNanos;
            if (elapsed <= 0 || frameId == 100_000) { reject(3); return; }
            event.traceId = traceId;
            event.frameId = ++frameId;
            event.intervalNanos = elapsed;
            if (enabled) event.commit();
        }
        lastNanos = now;
        previous = true;
    }

    @Name("nf.PresentInterval")
    @Label("NearFuture successful presentation interval")
    @Category("NearFuture")
    @StackTrace(false)
    public static final class Interval extends Event {
        public String traceId;
        public long frameId;
        public long intervalNanos;
        public long clockUncertaintyNanos;
    }
    @Name("nf.FrameProbeRejected")
    @Label("NearFuture presentation probe rejected")
    @Category("NearFuture")
    @StackTrace(false)
    public static final class Rejected extends Event { public int reason; }
    @Name("nf.ProbeConfiguration")
    @Label("NearFuture owned profiling configuration")
    @Category("NearFuture")
    @StackTrace(false)
    public static final class Configuration extends Event {
        public String traceId;
        public String classSha256;
        public boolean observerEnabled;
    }
}
