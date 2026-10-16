package nf.contract.system;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import nf.contract.system.SystemPackageTopology.Code;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPackageTopology.Topology;
import nf.contract.system.SystemPlacementRegistry.ConceptualSlot;
import nf.contract.system.SystemPlacementRegistry.Failure;
import nf.contract.system.SystemPlacementRegistry.PlacementEntry;
import nf.contract.system.SystemPlacementRegistry.PlacementError;
import nf.contract.system.SystemPlacementRegistry.ReservationOutcome;

/** Selected slot/capacity and error order on synthetic data, never publication authority. */
public final class SystemPlacementRegistryBoundsTest {
    private SystemPlacementRegistryBoundsTest() { }
    private static final String A1_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A2_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String B1_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333555555555555555555555555555555550100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A1_SHA = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
    private static final String A2_SHA = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
    private static final String B1_SHA = "6ea8fd3113fc299431367483e01f808086cdc28062ac9daf9d03a761fdcde821";

    private record Input(Topology topology, byte[] original, String hex, String hash) { }
    private record Row(Id history, Id owner, Id system, int slot, long revision,
            List<Byte> digest) { }
    private record Snapshot(Id universe, int capacity, List<Row> rows) { }

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
    private static Input input(String hex, String hash) {
        byte[] original = bytes(hex);
        return new Input(SystemPackageTopology.decode(original, SystemPackageTopology.DEFAULT),
                original, hex, hash);
    }
    private static void inputsUnchanged(List<Input> inputs) {
        check(inputs.size() == 3, "three fixed inputs");
        for (Input input : inputs) {
            check(input.original().length == 199, "fixed input size");
            check(Arrays.equals(input.original(), bytes(input.hex())), "original source bytes");
            SystemPackageTopology.validate(input.topology(), SystemPackageTopology.DEFAULT);
            check(Arrays.equals(SystemPackageTopology.encode(input.topology(),
                    SystemPackageTopology.DEFAULT), bytes(input.hex())), "complete input bytes");
            check(Arrays.equals(SystemPackageTopology.digest(input.topology(),
                    SystemPackageTopology.DEFAULT), bytes(input.hash())), "fixed input SHA");
        }
    }
    private static Snapshot snapshot(SystemPlacementRegistry registry) {
        List<PlacementEntry> entries = registry.entries();
        check(entries.size() <= 1, "capacity-one fixture snapshot");
        List<Row> rows = new ArrayList<>(entries.size());
        for (PlacementEntry entry : entries) {
            byte[] digest = entry.topologySha256();
            check(digest.length == 32, "exact digest size");
            List<Byte> ownedDigest = new ArrayList<>(32);
            for (byte value : digest) ownedDigest.add(value);
            rows.add(new Row(entry.history(), entry.owner(), entry.system(), entry.slot().value(),
                    entry.revision(), List.copyOf(ownedDigest)));
        }
        return new Snapshot(registry.universe(), registry.capacity(), List.copyOf(rows));
    }
    private static void assignment(SystemPlacementRegistry registry, int slot) {
        check(registry.universe().equals(id(0x11)), "namespace");
        check(registry.capacity() == 1, "capacity independent of slot");
        List<PlacementEntry> rows = registry.entries();
        check(rows.size() == 1, "one current row");
        PlacementEntry entry = rows.get(0);
        check(entry.history().equals(id(0x22)), "history");
        check(entry.owner().equals(id(0x33)), "owner");
        check(entry.system().equals(id(0x44)), "system");
        check(entry.slot().value() == slot, "literal slot");
        check(entry.revision() == 2, "seeded revision two");
        check(Arrays.equals(entry.topologySha256(), bytes(A2_SHA)), "fixed A2 digest");
    }
    private static void failure(PlacementError code, Code topologyCode, Runnable action,
            List<Input> inputs) {
        boolean refused = false;
        try {
            action.run();
        } catch (Failure error) {
            check(error.code() == code, "exact typed registry error");
            check(error.topologyCode() == topologyCode, "exact typed topology code");
            check(error.getCause() == null, "closed failure has no cause");
            refused = true;
        }
        check(refused, "ordinary refusal required");
        inputsUnchanged(inputs);
    }
    private static void refuses(SystemPlacementRegistry registry, PlacementError code,
            Code topologyCode, Runnable action, List<Input> inputs) {
        Snapshot before = snapshot(registry);
        failure(code, topologyCode, action, inputs);
        check(snapshot(registry).equals(before), "all owned registry fields unchanged");
    }
    private static void selectedSlotAndCapacityPrecedence() {
        Input a1 = input(A1_HEX, A1_SHA), a2 = input(A2_HEX, A2_SHA), b1 = input(B1_HEX, B1_SHA);
        List<Input> inputs = List.of(a1, a2, b1);
        inputsUnchanged(inputs);
        check(a1.topology().revision() == 1 && a2.topology().revision() == 2, "real stale input");
        check(b1.topology().system().equals(id(0x55)), "distinct new full key");
        Id namespace = id(0x11), zero = id(0);
        failure(PlacementError.INVALID_NAMESPACE, null,
                () -> { new SystemPlacementRegistry(null, 0); }, inputs);
        failure(PlacementError.INVALID_NAMESPACE, null,
                () -> { new SystemPlacementRegistry(zero, 257); }, inputs);
        failure(PlacementError.BOUNDS, null,
                () -> { new SystemPlacementRegistry(namespace, 0); }, inputs);
        failure(PlacementError.BOUNDS, null,
                () -> { new SystemPlacementRegistry(namespace, 257); }, inputs);
        check(Arrays.equals(namespace.value(), id(0x11).value()), "namespace input unchanged");
        check(Arrays.equals(zero.value(), id(0).value()), "zero namespace input unchanged");
        failure(PlacementError.BOUNDS, null, () -> { new ConceptualSlot(-1); }, inputs);
        failure(PlacementError.BOUNDS, null, () -> { new ConceptualSlot(65536); }, inputs);

        SystemPlacementRegistry full = new SystemPlacementRegistry(namespace, 1);
        ConceptualSlot lower = new ConceptualSlot(0), free = new ConceptualSlot(1);
        check(registryInsert(full, a2, lower) == ReservationOutcome.INSERTED, "slot zero usable");
        assignment(full, 0);
        SystemPlacementRegistry upper = new SystemPlacementRegistry(namespace, 1);
        check(registryInsert(upper, a2, new ConceptualSlot(65535)) == ReservationOutcome.INSERTED,
                "maximum slot usable at capacity one");
        assignment(upper, 65535);
        Snapshot upperBefore = snapshot(upper);

        SystemPlacementRegistry foreign = new SystemPlacementRegistry(id(0x66), 1);
        Snapshot emptyForeign = snapshot(foreign);
        refuses(foreign, PlacementError.TOPOLOGY, Code.INVALID,
                () -> foreign.reserve(a1.topology(), null, null), inputs);
        check(snapshot(foreign).equals(emptyForeign), "null limits preserve empty namespace");
        refuses(foreign, PlacementError.UNIVERSE_MISMATCH, null,
                () -> foreign.reserve(a1.topology(), null, SystemPackageTopology.DEFAULT), inputs);
        check(snapshot(foreign).equals(emptyForeign), "universe refusal preserves empty namespace");
        refuses(full, PlacementError.BOUNDS, null,
                () -> full.reserve(a2.topology(), null, SystemPackageTopology.DEFAULT), inputs);
        assignment(full, 0);

        refuses(full, PlacementError.SLOT_OCCUPIED, null,
                () -> full.reserve(b1.topology(), lower, SystemPackageTopology.DEFAULT), inputs);
        assignment(full, 0);
        check(full.entries().stream().noneMatch(entry -> entry.slot().value() == 1), "slot one free");
        refuses(full, PlacementError.BOUNDS, null,
                () -> full.reserve(b1.topology(), free, SystemPackageTopology.DEFAULT), inputs);
        assignment(full, 0);
        refuses(full, PlacementError.SLOT_MISMATCH, null,
                () -> full.reserve(a1.topology(), free, SystemPackageTopology.DEFAULT), inputs);
        assignment(full, 0);
        check(snapshot(upper).equals(upperBefore), "separate maximum-slot registry unchanged");
        assignment(upper, 65535);
        inputsUnchanged(inputs);
    }
    private static ReservationOutcome registryInsert(SystemPlacementRegistry registry, Input input,
            ConceptualSlot slot) {
        return registry.reserve(input.topology(), slot, SystemPackageTopology.DEFAULT);
    }
    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        selectedSlotAndCapacityPrecedence();
        System.out.println("NF-SPT-1 Java slot/capacity/error precedence conformance PASS; data only");
    }
}
