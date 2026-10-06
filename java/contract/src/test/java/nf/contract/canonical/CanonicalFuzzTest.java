package nf.contract.canonical;

import java.util.Arrays;

public final class CanonicalFuzzTest {
    private CanonicalFuzzTest() { }
    public static void main(String[] args) {
        Limits limits = new Limits(512, 32, 256, 8, 32, 8, 4096);
        byte[] profile = GoldenCorpus.hex(GoldenCorpus.POSITIVE.get(6).hex());
        byte[] record = GoldenCorpus.hex(GoldenCorpus.SEMANTIC.get(1).hex());
        long state = 0x51a7L;
        for (int i = 0; i < 20_000; i++) {
            state ^= state << 13; state ^= state >>> 7; state ^= state << 17;
            byte[] input = (i % 2 == 0 ? profile : record).clone();
            int index = (int) Long.remainderUnsigned(state, input.length);
            input[index] = (byte) (state >>> 32);
            if ((state & 3) == 0) input = Arrays.copyOf(input, index);
            try {
                byte[] reencoded = i % 2 == 0 ? Canonical.encodeProfile(Canonical.decodeProfile(input, limits), limits) : RecordCodec.encode(RecordCodec.decode(input, limits), limits);
                if (!Arrays.equals(input, reencoded)) throw new AssertionError("Accepted an alternative canonical encoding");
            } catch (Validation rejected) { }
        }
        Limits allocationLimit = new Limits(512, 32, 256, 8, 32, 8, 1);
        try { Canonical.decodeProfile(profile, allocationLimit); throw new AssertionError("Allocation budget ignored"); } catch (Validation expected) { }
        try { RecordCodec.decode(record, allocationLimit); throw new AssertionError("Allocation budget ignored"); } catch (Validation expected) { }
        System.out.println("PASS: 20000 finite canonical mutations/truncations with explicit byte/item/depth/allocation caps");
    }
}
