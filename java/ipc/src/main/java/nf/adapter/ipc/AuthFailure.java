package nf.adapter.ipc;
/** Bounded authentication failures never carry peer bytes or secret material. */
public final class AuthFailure extends RuntimeException {
    private static final long serialVersionUID=1L;
    public enum Code { INVALID, PROTOCOL, AUTHENTICATION, STATE }
    private final Code code;
    public AuthFailure(Code code) {super(code.name());this.code=code;}
    public Code code() {return code;}
}