package nf.adapter.ipc;

/** Static local-owner failures never retain command arguments, private paths or peer bytes. */
public final class NodeFailure extends RuntimeException {
    private static final long serialVersionUID = 1L;
    public enum Code { INVALID, PIN, STARTUP, PROTOCOL, AUTHENTICATION, CANCELLED, TIMEOUT, CHILD_EXIT, DISCONNECTED, CLEANUP }
    public enum StartupDetail { EXECUTABLE_IO, EXECUTABLE_ALGORITHM, CHILD_START_IO, ACTIVATION_IO }
    private final Code code;
    public NodeFailure(Code code) { super(code.name()); this.code = code; }
    public NodeFailure(StartupDetail detail) { super("STARTUP " + detail.name()); this.code = Code.STARTUP; }
    public Code code() { return code; }
}
