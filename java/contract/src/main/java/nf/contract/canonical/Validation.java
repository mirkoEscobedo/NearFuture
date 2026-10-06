package nf.contract.canonical;

/** Bounded public validation reasons; input content and private context are never included. */
public final class Validation extends IllegalArgumentException {
    private static final long serialVersionUID = 1L;
    public enum Code { TRUNCATED, UNKNOWN_RECORD, INVALID_PRESENCE, TRAILING_BYTES, INVALID_UTF8, NON_NFC, DUPLICATE_KEY, KEY_ORDER, LIMIT_EXCEEDED, INVALID_VALUE, UNKNOWN_REQUIRED_SEMANTIC, DIGEST_MISMATCH }
    private final Code code;
    public Validation(Code code) { super(code.name()); this.code = code; }
    public Code code() { return code; }
}
