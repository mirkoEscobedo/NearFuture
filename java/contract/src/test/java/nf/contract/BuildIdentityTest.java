package nf.contract;

public final class BuildIdentityTest {
    private BuildIdentityTest() { }

    public static void main(String[] args) {
        String expected = "Near Future 0.1.0 (synthetic; game integration unavailable)";
        if (!expected.equals(BuildIdentity.description())) {
            throw new AssertionError("Synthetic build identity must state game integration is unavailable");
        }
        System.out.println("PASS: Java synthetic build identity");
    }
}
