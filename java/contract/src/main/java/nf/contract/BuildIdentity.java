package nf.contract;

/** Dependency-free synthetic build identity; does not certify game integration. */
public final class BuildIdentity {
    private BuildIdentity() { }

    public static String description() {
        return "Near Future 0.1.0 (synthetic; game integration unavailable)";
    }
}
