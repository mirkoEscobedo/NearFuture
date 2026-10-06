package nf.contract.canonical;

import java.util.Arrays;
import java.util.HexFormat;

public final class CanonicalCorpusTest {
    private CanonicalCorpusTest() { }
    private static void match(String name, String hex, String hash, byte[] actual) {
        if (!Arrays.equals(GoldenCorpus.hex(hex), actual)) throw new AssertionError(name + ": canonical bytes differ");
        if (!hash.equals(HexFormat.of().formatHex(Canonical.sha256(actual)))) throw new AssertionError(name + ": SHA256 differs");
    }
    public static void main(String[] args) {
        for (GoldenCorpus.ProfileFixture fixture : GoldenCorpus.POSITIVE) {
            match(fixture.name(), fixture.hex(), fixture.hash(), Canonical.encodeProfile(fixture.record()));
            match(fixture.name(), fixture.hex(), fixture.hash(), Canonical.encodeProfile(Canonical.decodeProfile(GoldenCorpus.hex(fixture.hex()))));
        }
        for (GoldenCorpus.BadFixture fixture : GoldenCorpus.MALFORMED) {
            try { Canonical.decodeProfile(GoldenCorpus.hex(fixture.hex())); throw new AssertionError("Accepted malformed " + fixture.name()); }
            catch (Validation expected) { if (expected.getMessage().length() > 64) throw new AssertionError("Unbounded reason"); }
        }
        for (GoldenCorpus.RecordFixture fixture : GoldenCorpus.BINDINGS) {
            match(fixture.name(), fixture.hex(), fixture.hash(), RecordCodec.encode(fixture.record()));
            match(fixture.name(), fixture.hex(), fixture.hash(), RecordCodec.encode(RecordCodec.decode(GoldenCorpus.hex(fixture.hex()))));
        }
        for (GoldenCorpus.RecordFixture fixture : GoldenCorpus.SEMANTIC) {
            match(fixture.name(), fixture.hex(), fixture.hash(), RecordCodec.encode(fixture.record()));
            match(fixture.name(), fixture.hex(), fixture.hash(), RecordCodec.encode(RecordCodec.decode(GoldenCorpus.hex(fixture.hex()))));
        }
        for (GoldenCorpus.RequiredFixture fixture : GoldenCorpus.REQUIRED) {
            if (fixture.accepted()) {
                byte[] actual = RecordCodec.encode(fixture.record());
                if (!Arrays.equals(GoldenCorpus.hex(fixture.hex()), actual)) throw new AssertionError(fixture.name());
                RecordCodec.decode(actual);
            } else {
                try { RecordCodec.encode(fixture.record()); throw new AssertionError("Encoded " + fixture.name()); } catch (Validation expected) { }
                try { RecordCodec.decode(GoldenCorpus.hex(fixture.hex())); throw new AssertionError("Decoded " + fixture.name()); } catch (Validation expected) { }
            }
        }
        for (String value : new String[] {"-9223372036854775809", "9223372036854775808"}) {
            try { Long.parseLong(value); throw new AssertionError("i64 range accepted"); } catch (NumberFormatException expected) { }
        }
        for (String value : new String[] {"-1", "18446744073709551616"}) {
            try { Long.parseUnsignedLong(value); throw new AssertionError("u64 range accepted"); } catch (NumberFormatException expected) { }
        }
        try { Canonical.encodeProfile(new Canonical.Profile(null, "\ud800", java.util.List.of(), java.util.List.of())); throw new AssertionError("Surrogate accepted"); } catch (Validation expected) { }
        try { new Records.Id(new byte[15]); throw new AssertionError("Wrong ID width accepted"); } catch (Validation expected) { }
        System.out.println("PASS: independent canonical corpus " + GoldenCorpus.POSITIVE.size() + " positive/" + GoldenCorpus.MALFORMED.size() + " malformed/" + GoldenCorpus.BINDINGS.size() + " binding/" + GoldenCorpus.SEMANTIC.size() + " semantic/" + GoldenCorpus.REQUIRED.size() + " required, scalar boundaries");
    }
}
