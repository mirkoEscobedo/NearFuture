package nf.performance;

import java.util.concurrent.TimeUnit;
import jdk.jfr.FlightRecorder;
import jdk.jfr.RecordingState;
import org.lwjgl.opengl.Display;

/** Uses the VM's startup recording; it never starts an attach client or a second recording. */
public final class StartupPresentationFixture {
    private StartupPresentationFixture() { }
    public static void main(String[] args) throws Exception {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
        while (FlightRecorder.getFlightRecorder().getRecordings().stream()
            .noneMatch(recording -> recording.getName().equals("NF100") && recording.getState() == RecordingState.RUNNING)) {
            if (System.nanoTime() - deadline >= 0) throw new AssertionError("owned startup recording missing");
            Thread.sleep(1);
        }
        for (int call = 0; call < 4; call++) { Display.swapBuffers(); Thread.sleep(5); }
        if (Display.swaps != 4) throw new AssertionError("startup observation changed presentation effects");
        System.out.println("STARTUP_PRESENTATION_OK");
    }
}
