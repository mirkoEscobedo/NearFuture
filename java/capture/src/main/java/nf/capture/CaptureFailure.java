package nf.capture;
public final class CaptureFailure extends RuntimeException {
    private static final long serialVersionUID = 1L;
    public enum Code { INVALID, LIMIT, WRONG_THREAD, INACTIVE, SOURCE, DRIFT, CONFLICT, NATIVE_UNAVAILABLE }
    private final Code code;
    public CaptureFailure(Code code) { super(code.name()); this.code=code; }
    public Code code() { return code; }
}
