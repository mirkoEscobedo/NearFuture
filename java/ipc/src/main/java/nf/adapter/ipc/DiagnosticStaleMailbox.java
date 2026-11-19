package nf.adapter.ipc;

import jdk.jfr.Category;
import jdk.jfr.Event;
import jdk.jfr.Label;
import jdk.jfr.Name;
import jdk.jfr.StackTrace;

/** Actual retained-value publication results; contains only bounded local state. */
@Name("nf.DiagnosticStaleMailbox")
@Label("Diagnostic stale mailbox release")
@Category("NearFuture")
@StackTrace(false)
final class DiagnosticStaleMailbox extends Event {
    long oldEpoch;
    String state;
    boolean oldActive;
    boolean controlAccepted;
    boolean bulkAccepted;
}
