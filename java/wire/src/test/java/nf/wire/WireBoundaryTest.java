package nf.wire;

import java.util.HexFormat;

public final class WireBoundaryTest {
    private WireBoundaryTest() { }
    public static void main(String[] args) {
        byte[] duplicate = HexFormat.of().parseHex("08010801");
        try { ControlDecoder.decodeControl(duplicate); throw new AssertionError("Generated last-value behavior must not accept duplicate protocol fields"); }
        catch (WireFailure expected) {
            if (expected.code() != WireFailure.Code.DUPLICATE) throw new AssertionError(expected.code());
        }
        System.out.println("PASS: singular duplicates reject before generated decoding");
    }
}
