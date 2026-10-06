package nf.wire;

/** Fixed public failure classes; no tokens, paths, or private payloads are included. */
public final class WireFailure extends IllegalArgumentException {
    private static final long serialVersionUID = 1L;
    public enum Code { DUPLICATE, MALFORMED, UNKNOWN_FIELD, LIMIT, UNSUPPORTED, SEMANTIC }
    private final Code code;
    public WireFailure(Code code) { super(code.name()); this.code = code; }
    public Code code() { return code; }
}
