package nf.contract.system;

import java.util.Arrays;
import java.util.List;
import static nf.contract.system.SystemPackageTopology.*;

/** Independent NF-SPT-1 unsigned64 metadata vector; no shared-clock authority. */
public final class SystemPackageUnsignedMetadataTest {
    private SystemPackageUnsignedMetadataTest() { }
    private static final String GOLDEN = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440000000000000080ffffffffffffffff01010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String SHA = "840c1706efbc0e083b7743c01e9964f65b7698767e5bb5f983e7a2762b77fc53";

    private static byte[] repeated(int value, int count) {
        byte[] result = new byte[count];
        Arrays.fill(result, (byte) value);
        return result;
    }
    private static Id id(int value) { return new Id(repeated(value, 16)); }
    private static byte[] bytes(String hex) {
        if ((hex.length() & 1) != 0) throw new AssertionError("odd literal length");
        byte[] result = new byte[hex.length() / 2];
        for (int i = 0; i < result.length; i++) {
            result[i] = (byte) Integer.parseInt(hex.substring(i * 2, i * 2 + 2), 16);
        }
        return result;
    }
    private static void check(boolean condition) {
        if (!condition) throw new AssertionError("unsigned64 metadata vector");
    }

    public static void main(String[] args) {
        if (args.length != 0) throw new AssertionError("no arguments");
        // Signed long views of raw unsigned 2^63 and 2^64-1; neither is a clock grant.
        Topology expected = new Topology(id(0x11), id(0x22), id(0x33), id(0x44),
            Long.MIN_VALUE, -1L,
            List.of(new Dependency(new Key(repeated(0x77, 32)), true)),
            List.of(new Entity(id(1), Kind.STAR, null, null, "Sun"),
                new Entity(id(2), Kind.PLANET, id(1), 0, "World")));
        byte[] literal = bytes(GOLDEN);
        check(literal.length == 199);
        check(decode(literal, DEFAULT).equals(expected));
        check(Arrays.equals(encode(expected, DEFAULT), literal));
        check(Arrays.equals(digest(expected, DEFAULT), bytes(SHA)));
        System.out.println("NF-SPT-1 unsigned64 metadata vector PASS; descriptive data only");
    }
}
