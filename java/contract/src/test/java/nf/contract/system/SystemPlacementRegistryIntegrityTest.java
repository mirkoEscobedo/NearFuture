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

/** Selected pure-data integrity and ownership conformance, never publication authority. */
public final class SystemPlacementRegistryIntegrityTest {
    private SystemPlacementRegistryIntegrityTest() { }
    private static final String A1_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A1_SHA = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
    private static final String A2_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A2_SHA = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
    private static final String CONFLICT_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000030000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String CONFLICT_SHA = "52ed6079f4b31e1aa71f17a3740bfd085f56bc6bd8f4df1bb4ec31ae136fa5be";
    private static final String FOREIGN_HEX = "4e462d5350542d31000100666666666666666666666666666666662222222222222222222222222222222233333333333333333333333333333333555555555555555555555555555555550100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String FOREIGN_SHA = "2586c75cbccf4a306c74f25fcfbbdb0ce11e305dd8b5d4ac9bc903447845dcf6";
    private static final String SYSTEM_HIGH_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333808080808080808080808080808080800100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String SYSTEM_HIGH_SHA = "c2dacf84b51533e15caedd89d249aa45907d85771fd4e226bdb481fb6172e5d0";
    private static final String OWNER_HIGH_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222280808080808080808080808080808080444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String OWNER_HIGH_SHA = "7c84b1dcc28bcf9e396610d622dc76f18e94fde8f66fce8102d76cc4bcdc98cc";
    private static final String HISTORY_HIGH_HEX = "4e462d5350542d31000100111111111111111111111111111111118080808080808080808080808080808033333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String HISTORY_HIGH_SHA = "6af9fefed8d8d7a320c62fef00b3126a1285259245dbcea2dcb06fc4c7e01649";

    private record Input(Topology value, byte[] original, String hex, String sha) { }
    private record DependencyImage(List<Byte> key, boolean required) { }
    private record EntityImage(List<Byte> id, SystemPackageTopology.Kind kind,
            List<Byte> parent, Integer dependency, String label) { }
    private record TopologyImage(List<Byte> universe, List<Byte> history, List<Byte> owner,
            List<Byte> system, long revision, long day, List<DependencyImage> dependencies,
            List<EntityImage> entities) { }
    private record RowImage(List<Byte> history, List<Byte> owner, List<Byte> system,
            int slot, long revision, List<Byte> digest) { }
    private record RegistryImage(List<Byte> universe, int capacity, List<RowImage> rows) { }
    private record ExpectedRow(int history, int owner, int system, int slot,
            long revision, String sha) { }

    private static void check(boolean condition, String reason) {
        if (!condition) throw new AssertionError(reason);
    }
    private static byte[] bytes(String hex) {
        check(hex.length() == 398 || hex.length() == 64, "fixed literal size");
        byte[] result = new byte[hex.length() / 2];
        for (int i = 0; i < result.length; i++) {
            result[i] = (byte) Integer.parseInt(hex.substring(i * 2, i * 2 + 2), 16);
        }
        return result;
    }
    private static byte[] repeated(int value) {
        byte[] result = new byte[16];
        Arrays.fill(result, (byte) value);
        return result;
    }
    private static Id id(int value) { return new Id(repeated(value)); }
    private static List<Byte> owned(byte[] value) {
        check(value.length == 16 || value.length == 32, "fixed owned byte bound");
        List<Byte> result = new ArrayList<>(value.length);
        for (byte part : value) result.add(part);
        return List.copyOf(result);
    }
    private static List<Byte> idImage(Id value) {
        return value == null ? null : owned(value.value());
    }
    private static TopologyImage topologyImage(Topology value) {
        check(value.dependencies().size() <= 1 && value.entities().size() <= 2,
                "fixed topology snapshot bounds");
        List<DependencyImage> dependencies = new ArrayList<>(value.dependencies().size());
        for (SystemPackageTopology.Dependency dependency : value.dependencies()) {
            dependencies.add(new DependencyImage(owned(dependency.semanticKey().value()),
                    dependency.required()));
        }
        List<EntityImage> entities = new ArrayList<>(value.entities().size());
        for (SystemPackageTopology.Entity entity : value.entities()) {
            entities.add(new EntityImage(idImage(entity.id()), entity.kind(),
                    idImage(entity.parent()), entity.dependency(), entity.label()));
        }
        return new TopologyImage(idImage(value.universe()), idImage(value.history()),
                idImage(value.owner()), idImage(value.system()), value.revision(),
                value.campaignDayMillis(), List.copyOf(dependencies), List.copyOf(entities));
    }
    private static List<TopologyImage> inputImages(List<Topology> values) {
        check(values.size() <= 8, "fixed input snapshot count");
        List<TopologyImage> result = new ArrayList<>(values.size());
        for (Topology value : values) result.add(topologyImage(value));
        return List.copyOf(result);
    }
    private static RegistryImage registryImage(SystemPlacementRegistry registry) {
        List<PlacementEntry> entries = registry.entries();
        check(entries.size() <= 4, "four-key snapshot bound");
        List<RowImage> rows = new ArrayList<>(entries.size());
        for (PlacementEntry entry : entries) {
            rows.add(new RowImage(idImage(entry.history()), idImage(entry.owner()),
                    idImage(entry.system()), entry.slot().value(), entry.revision(),
                    owned(entry.topologySha256())));
        }
        return new RegistryImage(idImage(registry.universe()), registry.capacity(),
                List.copyOf(rows));
    }
    private static Input input(String hex, String sha) {
        byte[] original = bytes(hex);
        return new Input(SystemPackageTopology.decode(original, SystemPackageTopology.DEFAULT),
                original, hex, sha);
    }
    private static List<Input> fixedInputs() {
        return List.of(input(A1_HEX, A1_SHA), input(A2_HEX, A2_SHA),
                input(CONFLICT_HEX, CONFLICT_SHA), input(FOREIGN_HEX, FOREIGN_SHA),
                input(SYSTEM_HIGH_HEX, SYSTEM_HIGH_SHA), input(OWNER_HIGH_HEX, OWNER_HIGH_SHA),
                input(HISTORY_HIGH_HEX, HISTORY_HIGH_SHA));
    }
    private static void validInputsUnchanged(List<Input> inputs) {
        check(inputs.size() == 7, "seven fixed valid inputs");
        for (Input input : inputs) {
            check(input.original().length == 199, "canonical input size");
            check(Arrays.equals(input.original(), bytes(input.hex())), "original bytes unchanged");
            SystemPackageTopology.validate(input.value(), SystemPackageTopology.DEFAULT);
            check(Arrays.equals(SystemPackageTopology.encode(input.value(),
                    SystemPackageTopology.DEFAULT), bytes(input.hex())), "full canonical input");
            check(Arrays.equals(SystemPackageTopology.digest(input.value(),
                    SystemPackageTopology.DEFAULT), bytes(input.sha())), "independent input SHA");
        }
    }
    private static void assertRow(PlacementEntry entry, ExpectedRow expected) {
        check(entry.history().equals(id(expected.history())), "history");
        check(entry.owner().equals(id(expected.owner())), "owner");
        check(entry.system().equals(id(expected.system())), "system");
        check(entry.slot().value() == expected.slot(), "slot");
        check(entry.revision() == expected.revision(), "revision");
        check(Arrays.equals(entry.topologySha256(), bytes(expected.sha())), "fixed canonical SHA");
    }
    private static void assertRows(SystemPlacementRegistry registry, int capacity,
            List<ExpectedRow> expected) {
        check(registry.universe().equals(id(0x11)), "namespace");
        check(registry.capacity() == capacity, "capacity");
        List<PlacementEntry> entries = registry.entries();
        check(expected.size() <= 4 && entries.size() == expected.size(), "exact row count");
        for (int i = 0; i < expected.size(); i++) assertRow(entries.get(i), expected.get(i));
    }
    private static void insert(SystemPlacementRegistry registry, Input input, int slot,
            List<Input> inputs) {
        check(registry.reserve(input.value(), new ConceptualSlot(slot),
                SystemPackageTopology.DEFAULT) == ReservationOutcome.INSERTED, "inserted");
        validInputsUnchanged(inputs);
    }
    private static void refuses(SystemPlacementRegistry registry, PlacementError code,
            Code topologyCode, Runnable action, List<Input> inputs, List<Topology> values) {
        RegistryImage before = registryImage(registry);
        List<TopologyImage> inputsBefore = inputImages(values);
        boolean refused = false;
        try {
            action.run();
        } catch (Failure error) {
            check(error.code() == code, "exact registry failure");
            check(error.topologyCode() == topologyCode, "exact topology code");
            check(error.getCause() == null, "closed failure without cause");
            refused = true;
        }
        check(refused, "ordinary refusal required");
        check(registryImage(registry).equals(before), "whole registry unchanged");
        check(inputImages(values).equals(inputsBefore), "all owned input fields unchanged");
        validInputsUnchanged(inputs);
    }
    private static void conflictValidationAndNonmutation(List<Input> inputs) {
        Input a1 = inputs.get(0), a2 = inputs.get(1), conflict = inputs.get(2);
        Topology foreign = inputs.get(3).value();
        // These constructors do not validate; only reserve is inside each refusal action.
        Topology invalid = new Topology(foreign.universe(), foreign.history(), foreign.owner(),
                foreign.system(), 0, foreign.campaignDayMillis(), foreign.dependencies(),
                foreign.entities());
        SystemPackageTopology.Limits invalidLimits =
                new SystemPackageTopology.Limits(129, 64, 128, 65536);
        List<Topology> values = new ArrayList<>(8);
        for (Input input : inputs) values.add(input.value());
        values.add(invalid);
        values = List.copyOf(values);
        check(invalid.revision() == 0 && invalid.universe().equals(id(0x66)),
                "constructed invalid value reaches reserve");
        check(foreign.system().equals(id(0x55)), "competing new key");
        check(a2.value().revision() == 2 && conflict.value().revision() == 2
                && conflict.value().campaignDayMillis() == 3, "real equal-revision conflict");
        check(a1.value().revision() == 1, "real lower revision");
        SystemPlacementRegistry registry = new SystemPlacementRegistry(id(0x11), 1);
        insert(registry, a2, 7, inputs);
        List<ExpectedRow> expected = List.of(new ExpectedRow(0x22, 0x33, 0x44, 7, 2, A2_SHA));
        assertRows(registry, 1, expected);
        ConceptualSlot occupied = new ConceptualSlot(7);
        refuses(registry, PlacementError.REVISION_CONFLICT, null,
                () -> registry.reserve(conflict.value(), occupied, SystemPackageTopology.DEFAULT),
                inputs, values);
        assertRows(registry, 1, expected);
        refuses(registry, PlacementError.STALE_REVISION, null,
                () -> registry.reserve(a1.value(), occupied, SystemPackageTopology.DEFAULT),
                inputs, values);
        assertRows(registry, 1, expected);
        refuses(registry, PlacementError.TOPOLOGY, Code.INVALID,
                () -> registry.reserve(invalid, occupied, SystemPackageTopology.DEFAULT),
                inputs, values);
        assertRows(registry, 1, expected);
        refuses(registry, PlacementError.TOPOLOGY, Code.LIMIT,
                () -> registry.reserve(foreign, occupied, invalidLimits), inputs, values);
        assertRows(registry, 1, expected);
        refuses(registry, PlacementError.UNIVERSE_MISMATCH, null,
                () -> registry.reserve(foreign, occupied, SystemPackageTopology.DEFAULT),
                inputs, values);
        assertRows(registry, 1, expected);
        check(invalidLimits.equals(new SystemPackageTopology.Limits(129, 64, 128, 65536)),
                "limits input unchanged");
    }
    private static List<ExpectedRow> initialKeyRows() {
        // Independent unsigned history -> owner -> system order, never sorted by product output.
        return List.of(new ExpectedRow(0x22, 0x33, 0x44, 9, 1, A1_SHA),
                new ExpectedRow(0x22, 0x33, 0x80, 3, 1, SYSTEM_HIGH_SHA),
                new ExpectedRow(0x22, 0x80, 0x44, 12, 1, OWNER_HIGH_SHA),
                new ExpectedRow(0x80, 0x33, 0x44, 5, 1, HISTORY_HIGH_SHA));
    }
    private static void canonicalKeyAndDefensiveOwnership(List<Input> inputs) {
        byte[] namespaceSource = repeated(0x11);
        Id namespace = new Id(namespaceSource);
        SystemPlacementRegistry left = new SystemPlacementRegistry(namespace, 4);
        SystemPlacementRegistry right = new SystemPlacementRegistry(id(0x11), 4);
        insert(left, inputs.get(6), 5, inputs);
        insert(left, inputs.get(4), 3, inputs);
        insert(left, inputs.get(5), 12, inputs);
        insert(left, inputs.get(0), 9, inputs);
        insert(right, inputs.get(0), 9, inputs);
        insert(right, inputs.get(5), 12, inputs);
        insert(right, inputs.get(4), 3, inputs);
        insert(right, inputs.get(6), 5, inputs);
        List<ExpectedRow> expected = initialKeyRows();
        assertRows(left, 4, expected);
        assertRows(right, 4, expected);
        RegistryImage before = registryImage(left), rightBefore = registryImage(right);
        check(before.equals(rightBefore), "insertion order independence");
        List<PlacementEntry> oldRows = left.entries();
        PlacementEntry oldFirst = oldRows.get(0);
        List<Byte> oldHash = owned(oldFirst.topologySha256());

        namespaceSource[0] ^= (byte) 0x80;
        check(namespace.equals(id(0x11)), "caller namespace array is not retained");
        check(registryImage(left).equals(before), "namespace-source mutation preserves registry");
        byte[] historySource = repeated(0x22);
        Id history = new Id(historySource);
        Topology a1 = inputs.get(0).value();
        Topology copied = new Topology(a1.universe(), history, a1.owner(), a1.system(),
                a1.revision(), a1.campaignDayMillis(), a1.dependencies(), a1.entities());
        TopologyImage copiedBefore = topologyImage(copied);
        historySource[0] ^= (byte) 0x80;
        check(topologyImage(copied).equals(copiedBefore), "caller Id source does not alter input");
        check(registryImage(left).equals(before), "caller Id source preserves registry");
        check(left.reserve(copied, new ConceptualSlot(9), SystemPackageTopology.DEFAULT)
                == ReservationOutcome.UNCHANGED, "owned Id has original canonical meaning");
        check(topologyImage(copied).equals(copiedBefore), "copied input unchanged by reserve");
        check(Arrays.equals(SystemPackageTopology.encode(copied, SystemPackageTopology.DEFAULT),
                bytes(A1_HEX)), "copied input fixed canonical bytes");
        check(registryImage(left).equals(before), "unchanged admission preserves full registry");

        byte[] returnedHash = oldFirst.topologySha256();
        returnedHash[0] ^= (byte) 0x80;
        check(owned(oldFirst.topologySha256()).equals(oldHash), "entry owns its hash");
        check(registryImage(left).equals(before), "returned hash mutation preserves registry");
        List<byte[]> returnedIds = List.of(left.universe().value(), oldFirst.history().value(),
                oldFirst.owner().value(), oldFirst.system().value());
        for (byte[] returned : returnedIds) returned[0] ^= (byte) 0x80;
        assertRow(oldFirst, expected.get(0));
        check(registryImage(left).equals(before), "all returned Id arrays are defensive");
        boolean refused = false;
        try {
            oldRows.clear();
        } catch (UnsupportedOperationException error) {
            check(error.getClass() == UnsupportedOperationException.class,
                    "exact immutable list exception type");
            refused = true;
        }
        check(refused, "snapshot list must reject mutation");
        check(registryImage(left).equals(before), "list refusal preserves registry");
        validInputsUnchanged(inputs);

        check(left.reserve(inputs.get(1).value(), new ConceptualSlot(9),
                SystemPackageTopology.DEFAULT) == ReservationOutcome.UPDATED, "later real update");
        List<ExpectedRow> updated = List.of(new ExpectedRow(0x22, 0x33, 0x44, 9, 2, A2_SHA),
                expected.get(1), expected.get(2), expected.get(3));
        assertRows(left, 4, updated);
        check(oldRows.size() == 4, "old list remains complete");
        for (int i = 0; i < 4; i++) assertRow(oldRows.get(i), expected.get(i));
        assertRow(oldFirst, expected.get(0));
        check(owned(oldFirst.topologySha256()).equals(oldHash), "old hash survives later update");
        assertRows(right, 4, expected);
        check(registryImage(right).equals(rightBefore), "other model unchanged by update");
        check(topologyImage(copied).equals(copiedBefore), "owned copied input remains unchanged");
        validInputsUnchanged(inputs);
    }
    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        List<Input> inputs = fixedInputs();
        validInputsUnchanged(inputs);
        conflictValidationAndNonmutation(inputs);
        canonicalKeyAndDefensiveOwnership(inputs);
        System.out.println("NF-SPT-1 Java integrity conformance PASS; data only");
    }
}
