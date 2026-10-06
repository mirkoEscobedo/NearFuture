package nf.contract.canonical;

public final class Ed25519Test {
    private Ed25519Test() { }
    public static void main(String[] args) {
        for (GoldenCorpus.SignatureFixture fixture : GoldenCorpus.SIGNATURES) {
            if (!Ed25519.verify(GoldenCorpus.hex(fixture.publicKey()), GoldenCorpus.hex(fixture.digest()), GoldenCorpus.hex(fixture.signature()))) throw new AssertionError("Canonical digest signature mismatch: " + fixture.name());
            byte[] expected = GoldenCorpus.hex(fixture.signature());
            if (!java.util.Arrays.equals(expected, Ed25519.sign(GoldenCorpus.hex(fixture.seed()), GoldenCorpus.hex(fixture.digest())))) throw new AssertionError("Independent deterministic signature differs");
            byte[] modified = GoldenCorpus.hex(fixture.digest()); modified[0] ^= 1;
            if (Ed25519.verify(GoldenCorpus.hex(fixture.publicKey()), modified, expected)) throw new AssertionError("Modified canonical digest accepted");
            expected[0] ^= 1;
            if (Ed25519.verify(GoldenCorpus.hex(fixture.publicKey()), GoldenCorpus.hex(fixture.digest()), expected)) throw new AssertionError("Modified signature accepted");
        }
        if (Ed25519.verify(new byte[31], new byte[32], new byte[64])) throw new AssertionError("Wrong key width accepted");
        if (Ed25519.verify(new byte[32], new byte[31], new byte[64])) throw new AssertionError("Wrong digest width accepted");
        if (Ed25519.verify(new byte[32], new byte[32], new byte[65])) throw new AssertionError("Wrong signature width accepted");
        for (GoldenCorpus.BadSignatureFixture fixture : GoldenCorpus.NEGATIVE_SIGNATURES) {
            if (Ed25519.verify(GoldenCorpus.hex(fixture.publicKey()), GoldenCorpus.hex(fixture.digest()), GoldenCorpus.hex(fixture.signature()))) throw new AssertionError(fixture.name());
        }
        byte[] identity = new byte[32]; identity[0] = 1;
        byte[] forged = new byte[64]; forged[0] = 1;
        if (Ed25519.verify(identity, new byte[32], forged)) throw new AssertionError("Identity-point forgery accepted for arbitrary digest");
        if (Ed25519.verify(GoldenCorpus.hex(GoldenCorpus.SIGNATURES.get(0).publicKey()), new byte[32], forged)) throw new AssertionError("Weak signature R accepted");
        System.out.println("PASS: maintained Ed25519, 2 independent signatures and 14 strict-point negatives");
    }
}
