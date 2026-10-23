package nf.adapter.ipc;

import jdk.jfr.Category;
import jdk.jfr.Event;
import jdk.jfr.Label;
import jdk.jfr.Name;
import jdk.jfr.StackTrace;

/** Bounded worker-only receipt: closed delay and outcome, with real JFR duration. */
@Name("nf.DiagnosticHandoffDelay")
@Label("Diagnostic handoff delay")
@Category("NearFuture")
@StackTrace(false)
final class DiagnosticHandoffDelay extends Event {
    long requestedMillis;
    String outcome;
}
