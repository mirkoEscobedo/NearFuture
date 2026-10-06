package nf.contract.canonical;

import java.util.Arrays;
import java.util.HexFormat;
import java.util.List;

public final class CanonicalProfileTest {
    private CanonicalProfileTest() { }
    public static void main(String[] args) {
        byte[] expected = HexFormat.of().parseHex("4e462d43414e4f4e2d3100ff000100010000000000000000000000");
        Canonical.Profile profile = new Canonical.Profile(null, null, List.of(), List.of());
        if (!Arrays.equals(expected, Canonical.encodeProfile(profile))) {
            throw new AssertionError("Absent optionals and empty collections must retain explicit header/presence");
        }
        byte[] present = HexFormat.of().parseHex("4e462d43414e4f4e2d3100ff00010001000001000000000000000000000000");
        Canonical.Profile presentText = new Canonical.Profile(null, "", List.of(), List.of());
        if (!Arrays.equals(present, Canonical.encodeProfile(presentText))) {
            throw new AssertionError("Present empty text must differ from absent");
        }
        System.out.println("PASS: canonical empty profile");
    }
}
