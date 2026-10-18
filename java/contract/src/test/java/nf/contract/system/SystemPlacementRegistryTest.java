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

/** One independent public collision/update scenario. No publication or game authority. */
public final class SystemPlacementRegistryTest {
    private SystemPlacementRegistryTest() { }
    private static final String A1_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A2_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String B1_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333555555555555555555555555555555550100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A1_SHA = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
    private static final String A2_SHA = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
    private static final String B1_SHA = "6ea8fd3113fc299431367483e01f808086cdc28062ac9daf9d03a761fdcde821";

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
    private static void inputUnchanged(Topology topology, byte[] literal, String expectedHash) {
        check(literal.length == 199, "199-byte input");
        SystemPackageTopology.validate(topology, SystemPackageTopology.DEFAULT);
        check(Arrays.equals(SystemPackageTopology.encode(topology, SystemPackageTopology.DEFAULT), literal), "input bytes");
        check(Arrays.equals(SystemPackageTopology.digest(topology, SystemPackageTopology.DEFAULT), bytes(expectedHash)), "input digest");
    }
    private static void row(List<PlacementEntry> rows, long revision, String expectedHash) {
        check(rows.size() == 1, "one current row");
        PlacementEntry entry = rows.get(0);
        check(entry.history().equals(id(0x22)), "history");
        check(entry.owner().equals(id(0x33)), "owner");
        check(entry.system().equals(id(0x44)), "system");
        check(entry.slot().value() == 7, "stable slot");
        check(entry.revision() == revision, "current revision");
        check(Arrays.equals(entry.topologySha256(), bytes(expectedHash)), "current literal hash");
    }
    private static void assignment(SystemPlacementRegistry registry, long revision, String expectedHash) {
        check(registry.universe().equals(id(0x11)), "registry namespace");
        check(registry.capacity() == 2, "independent row capacity");
        row(registry.entries(), revision, expectedHash);
    }
    private static void slotCollisionRefusesWithoutChangingExistingAssignment() {
        byte[] a1Bytes = bytes(A1_HEX), a2Bytes = bytes(A2_HEX), b1Bytes = bytes(B1_HEX);
        Topology a1 = SystemPackageTopology.decode(a1Bytes, SystemPackageTopology.DEFAULT);
        Topology a2 = SystemPackageTopology.decode(a2Bytes, SystemPackageTopology.DEFAULT);
        Topology b1 = SystemPackageTopology.decode(b1Bytes, SystemPackageTopology.DEFAULT);
        inputUnchanged(a1, a1Bytes, A1_SHA);
        inputUnchanged(a2, a2Bytes, A2_SHA);
        inputUnchanged(b1, b1Bytes, B1_SHA);
        check(b1.system().equals(id(0x55)), "distinct collision key");
        SystemPlacementRegistry registry = new SystemPlacementRegistry(id(0x11), 2);
        ConceptualSlot slot = new ConceptualSlot(7);
        check(registry.reserve(a1, slot, SystemPackageTopology.DEFAULT) == ReservationOutcome.INSERTED, "insert A1");
        assignment(registry, 1, A1_SHA);
        List<PlacementEntry> insertedRows = registry.entries();
        check(registry.reserve(a2, slot, SystemPackageTopology.DEFAULT) == ReservationOutcome.UPDATED, "update A2");
        assignment(registry, 2, A2_SHA);
        row(insertedRows, 1, A1_SHA);
        List<PlacementEntry> beforeCollision = registry.entries();
        boolean refused = false;
        try {
            registry.reserve(b1, slot, SystemPackageTopology.DEFAULT);
        } catch (Failure error) {
            check(error.code() == PlacementError.SLOT_OCCUPIED, "typed slot collision");
            check(error.topologyCode() == null, "collision is not topology refusal");
            refused = true;
        }
        check(refused, "collision must refuse");
        check(registry.entries().equals(beforeCollision), "whole row snapshot unchanged");
        assignment(registry, 2, A2_SHA);
        row(insertedRows, 1, A1_SHA);
        inputUnchanged(a1, a1Bytes, A1_SHA);
        inputUnchanged(a2, a2Bytes, A2_SHA);
        inputUnchanged(b1, b1Bytes, B1_SHA);
        check(Arrays.equals(a1Bytes, bytes(A1_HEX)), "original A1 literal preserved");
        check(Arrays.equals(a2Bytes, bytes(A2_HEX)), "original A2 literal preserved");
        check(Arrays.equals(b1Bytes, bytes(B1_HEX)), "original B1 literal preserved");
    }
    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        slotCollisionRefusesWithoutChangingExistingAssignment();
        System.out.println("NF-SPT-1 Java slot collision/update/nonmutation PASS; data only");
    }
}
