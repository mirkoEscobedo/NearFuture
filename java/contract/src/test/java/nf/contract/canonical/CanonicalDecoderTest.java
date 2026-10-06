package nf.contract.canonical;

import java.util.HexFormat;

public final class CanonicalDecoderTest {
    private CanonicalDecoderTest() { }
    public static void main(String[] args) {
        byte[] malicious = HexFormat.of().parseHex("4e462d43414e4f4e2d3100ff000100010001ffffffff");
        try {
            Canonical.decodeProfile(malicious);
            throw new AssertionError("An over-limit declared length must reject before allocation");
        } catch (Validation expected) {
            if (expected.code() != Validation.Code.LIMIT_EXCEEDED) throw new AssertionError(expected.code());
        }
        System.out.println("PASS: bounded canonical lengths");
    }
}
