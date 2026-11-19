package nf.adapter.ipc;

import jdk.jfr.Category;
import jdk.jfr.Event;
import jdk.jfr.Label;
import jdk.jfr.Name;
import jdk.jfr.StackTrace;

/** One closed outcome after each real worker attempt's fence and owned cleanup. */
@Name("nf.OwnerAttempt")
@Label("Local owner attempt")
@Category("NearFuture")
@StackTrace(false)
final class DiagnosticOwnerAttempt extends Event {
    long generation;
    int attempt;
    long operationEpoch;
    long terminalEpoch;
    String phase;
    String code;
    String detail;
    boolean childCreated;
    long childPid;
    boolean exitObserved;
    int exitStatus;
    String cleanup;
    String cleanupFailure;

}
