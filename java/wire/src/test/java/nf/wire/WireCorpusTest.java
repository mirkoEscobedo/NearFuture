package nf.wire;

import java.util.HexFormat;

public final class WireCorpusTest {
    private WireCorpusTest() { }
    public static void main(String[] args) {
        for (WireCorpus.Fixture fixture : WireCorpus.POSITIVE) ControlDecoder.decodeControl(HexFormat.of().parseHex(fixture.hex()));
        for (WireCorpus.Fixture fixture : WireCorpus.MALFORMED) {
            try { ControlDecoder.decodeControl(HexFormat.of().parseHex(fixture.hex())); throw new AssertionError("Accepted " + fixture.name()); }
            catch (WireFailure rejected) { if (!rejected.code().name().equals(fixture.failure())) throw new AssertionError(fixture.name() + ": " + rejected.code() + ", expected " + fixture.failure()); }
        }
        for (WireCorpus.Fixture fixture : WireCorpus.FRAME_MALFORMED) {
            try { ControlDecoder.decodeFrame(HexFormat.of().parseHex(fixture.hex())); throw new AssertionError("Accepted " + fixture.name()); }
            catch (WireFailure rejected) { if (!rejected.code().name().equals(fixture.failure())) throw new AssertionError(fixture.name() + ": " + rejected.code()); }
        }
        System.out.println("PASS: shared independent wire corpus " + WireCorpus.POSITIVE.size() + " positive/" + WireCorpus.MALFORMED.size() + " malformed/3 framing");
    }
}
