package nf.contract.system;

import java.util.Arrays;
import java.util.List;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPackageTopology.Topology;
import nf.contract.system.SystemPlacementRegistry.ConceptualSlot;
import nf.contract.system.SystemPlacementRegistry.Failure;
import nf.contract.system.SystemPlacementRegistry.PlacementEntry;
import nf.contract.system.SystemPlacementRegistry.PlacementError;
import nf.contract.system.SystemPlacementRegistry.ReservationOutcome;

/** One unsigned revision family on actual registry inputs, without publication authority. */
public final class SystemPlacementRegistryUnsignedTest {
    private SystemPlacementRegistryUnsignedTest() { }
    private static final String A2_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String HIGH_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440000000000000080ffffffffffffffff01010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String MAX_HEX = "4e462d5350542d3100010011111111111111111111111111111111222222222222222222222222222222223333333333333333333333333333333344444444444444444444444444444444ffffffffffffffffffffffffffffffff01010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A2_SHA = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
    private static final String HIGH_SHA = "840c1706efbc0e083b7743c01e9964f65b7698767e5bb5f983e7a2762b77fc53";
    private static final String MAX_SHA = "af7535b29a044a03adf92107775dd55b2c4e14f03fac34ecca4e216d53151345";

    private static void check(boolean condition, String reason) {
        if (!condition) throw new AssertionError(reason);
    }
    private static byte[] bytes(String hex) {
        check(hex.length() == 398 || hex.length() == 64, "fixed literal length");
        byte[] result = new byte[hex.length() / 2];
        for (int i = 0; i < result.length; i++) {
            result[i] = (byte) Integer.parseInt(hex.substring(i * 2, i * 2 + 2), 16);
        }
        return result;
    }
    private static Id id(int value) {
        byte[] result = new byte[16];
        Arrays.fill(result, (byte) value);
        return new Id(result);
    }
    private static void inputUnchanged(Topology topology, byte[] original, String hex,
            String hash, long revision, long date) {
        check(original.length == 199 && Arrays.equals(original, bytes(hex)), "original literal bytes");
        check(topology.revision() == revision, "input unsigned revision bits");
        check(topology.campaignDayMillis() == date, "input descriptive date bits");
        SystemPackageTopology.validate(topology, SystemPackageTopology.DEFAULT);
        check(Arrays.equals(SystemPackageTopology.encode(topology, SystemPackageTopology.DEFAULT), bytes(hex)), "input serialization");
        check(Arrays.equals(SystemPackageTopology.digest(topology, SystemPackageTopology.DEFAULT), bytes(hash)), "input literal digest");
    }
    private static void row(List<PlacementEntry> rows, long revision, String hash) {
        check(rows.size() == 1, "one current row");
        PlacementEntry entry = rows.get(0);
        check(entry.history().equals(id(0x22)), "history");
        check(entry.owner().equals(id(0x33)), "owner");
        check(entry.system().equals(id(0x44)), "system");
        check(entry.slot().value() == 7, "stable slot");
        check(entry.revision() == revision, "stored unsigned revision bits");
        check(Arrays.equals(entry.topologySha256(), bytes(hash)), "stored literal digest");
    }
    private static void assignment(SystemPlacementRegistry registry, long revision, String hash) {
        check(registry.universe().equals(id(0x11)), "registry namespace");
        check(registry.capacity() == 2, "independent row capacity");
        row(registry.entries(), revision, hash);
    }
    private static void stale(SystemPlacementRegistry registry, Topology older,
            ConceptualSlot slot, List<PlacementEntry> maximumRows) {
        boolean refused = false;
        try {
            registry.reserve(older, slot, SystemPackageTopology.DEFAULT);
        } catch (Failure error) {
            check(error.code() == PlacementError.STALE_REVISION, "typed stale revision");
            check(error.topologyCode() == null, "not a topology refusal");
            refused = true;
        }
        check(refused, "older unsigned revision must refuse");
        check(registry.entries().equals(maximumRows), "whole maximum row snapshot unchanged");
        assignment(registry, -1L, MAX_SHA);
    }
    private static void unsignedRevisionsPreserveCurrentMaximum() {
        byte[] a2Bytes = bytes(A2_HEX), highBytes = bytes(HIGH_HEX), maxBytes = bytes(MAX_HEX);
        Topology a2 = SystemPackageTopology.decode(a2Bytes, SystemPackageTopology.DEFAULT);
        Topology high = SystemPackageTopology.decode(highBytes, SystemPackageTopology.DEFAULT);
        Topology maximum = SystemPackageTopology.decode(maxBytes, SystemPackageTopology.DEFAULT);
        inputUnchanged(a2, a2Bytes, A2_HEX, A2_SHA, 2, 2);
        inputUnchanged(high, highBytes, HIGH_HEX, HIGH_SHA, Long.MIN_VALUE, -1L);
        inputUnchanged(maximum, maxBytes, MAX_HEX, MAX_SHA, -1L, -1L);
        SystemPlacementRegistry registry = new SystemPlacementRegistry(id(0x11), 2);
        ConceptualSlot slot = new ConceptualSlot(7);
        check(registry.reserve(a2, slot, SystemPackageTopology.DEFAULT) == ReservationOutcome.INSERTED, "insert revision2");
        assignment(registry, 2, A2_SHA);
        List<PlacementEntry> revisionTwoRows = registry.entries();
        check(registry.reserve(high, slot, SystemPackageTopology.DEFAULT) == ReservationOutcome.UPDATED, "update unsigned2pow63");
        assignment(registry, Long.MIN_VALUE, HIGH_SHA);
        row(revisionTwoRows, 2, A2_SHA);
        List<PlacementEntry> highRows = registry.entries();
        check(registry.reserve(maximum, slot, SystemPackageTopology.DEFAULT) == ReservationOutcome.UPDATED, "update unsignedMAX");
        assignment(registry, -1L, MAX_SHA);
        row(revisionTwoRows, 2, A2_SHA);
        row(highRows, Long.MIN_VALUE, HIGH_SHA);
        List<PlacementEntry> maximumRows = registry.entries();
        check(registry.reserve(maximum, slot, SystemPackageTopology.DEFAULT) == ReservationOutcome.UNCHANGED, "repeat exactMAX");
        check(registry.entries().equals(maximumRows), "repeat maximum whole snapshot");
        assignment(registry, -1L, MAX_SHA);
        stale(registry, high, slot, maximumRows);
        stale(registry, a2, slot, maximumRows);
        row(revisionTwoRows, 2, A2_SHA);
        row(highRows, Long.MIN_VALUE, HIGH_SHA);
        row(maximumRows, -1L, MAX_SHA);
        inputUnchanged(a2, a2Bytes, A2_HEX, A2_SHA, 2, 2);
        inputUnchanged(high, highBytes, HIGH_HEX, HIGH_SHA, Long.MIN_VALUE, -1L);
        inputUnchanged(maximum, maxBytes, MAX_HEX, MAX_SHA, -1L, -1L);
    }
    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        unsignedRevisionsPreserveCurrentMaximum();
        System.out.println("NF-SPT-1 Java unsigned registry revision conformance PASS; data only");
    }
}
