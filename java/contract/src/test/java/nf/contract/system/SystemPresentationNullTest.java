package nf.contract.system;

import java.util.Arrays;
import java.util.HexFormat;
import java.util.List;
import nf.contract.system.SystemPackageTopology.Dependency;
import nf.contract.system.SystemPackageTopology.Entity;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPackageTopology.Topology;
import nf.contract.system.SystemPlacementRegistry.PlacementEntry;
import nf.contract.system.SystemPresentation.Domain;
import nf.contract.system.SystemPresentation.Profile;
import nf.contract.system.SystemPublicationHistory.PublicationRecord;

/** Nullable project arguments refuse before foreign namespace without source mutation; data only. */
public final class SystemPresentationNullTest {
    private SystemPresentationNullTest() { }

    private static final String A1_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A2_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A1_SHA = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
    private static final String A2_SHA = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";

    private static final boolean[] NULL_REGISTRY = {true, false, false, false};
    private static final String[][] QUERY_HEX = {
        {"22222222222222222222222222222222", "33333333333333333333333333333333", "44444444444444444444444444444444"},
        {null, "33333333333333333333333333333333", "44444444444444444444444444444444"},
        {"22222222222222222222222222222222", null, "44444444444444444444444444444444"},
        {"22222222222222222222222222222222", "33333333333333333333333333333333", null}
    };

    private record Fixture(Id universe, List<Id> query, List<Domain> domains,
            List<Topology> topologies, byte[][] originals, SystemPackageTopology.Limits limits) { }

    private static void check(boolean condition, String reason) {
        if (!condition) throw new AssertionError(reason);
    }

    private static byte[] bytes(String hex) { return HexFormat.of().parseHex(hex); }

    private static Id id(int value) {
        byte[] raw = new byte[16];
        Arrays.fill(raw, (byte) value);
        return new Id(raw);
    }

    private static byte[] idImage(Id value) { return value == null ? null : value.value().clone(); }

    private static Object[] domainImage(Domain domain) {
        return new Object[] {domain.originX(), domain.originY(), domain.width(),
            domain.height(), domain.halfExtent()};
    }

    private static Object[] topologyImage(Topology value) {
        Object[][] dependencies = new Object[value.dependencies().size()][];
        for (int i = 0; i < dependencies.length; i++) {
            Dependency d = value.dependencies().get(i);
            dependencies[i] = new Object[] {d.semanticKey().value().clone(), d.required()};
        }
        Object[][] entities = new Object[value.entities().size()][];
        for (int i = 0; i < entities.length; i++) {
            Entity e = value.entities().get(i);
            entities[i] = new Object[] {idImage(e.id()), e.kind(), idImage(e.parent()),
                e.dependency(), e.label()};
        }
        return new Object[] {idImage(value.universe()), idImage(value.history()), idImage(value.owner()),
            idImage(value.system()), value.revision(), value.campaignDayMillis(), dependencies, entities};
    }

    private static Object[] inputImage(Fixture fixture) {
        Object[][] topologies = new Object[2][];
        for (int i = 0; i < 2; i++) topologies[i] = topologyImage(fixture.topologies().get(i));
        return new Object[] {idImage(fixture.universe()), idImage(fixture.query().get(0)),
            idImage(fixture.query().get(1)), idImage(fixture.query().get(2)),
            domainImage(fixture.domains().get(0)), domainImage(fixture.domains().get(1)), topologies,
            fixture.originals()[0].clone(), fixture.originals()[1].clone(),
            fixture.limits().entities(), fixture.limits().dependencies(),
            fixture.limits().labelBytes(), fixture.limits().documentBytes()};
    }

    private static Object[] entryImage(PlacementEntry entry) {
        return new Object[] {idImage(entry.history()), idImage(entry.owner()), idImage(entry.system()),
            entry.slot().value(), entry.revision(), entry.topologySha256().clone()};
    }

    private static Object[] registryImage(SystemPlacementRegistry registry) {
        List<PlacementEntry> entries = registry.entries();
        Object[][] rows = new Object[entries.size()][];
        for (int i = 0; i < rows.length; i++) rows[i] = entryImage(entries.get(i));
        return new Object[] {idImage(registry.universe()), registry.capacity(), rows};
    }

    private static Object[] recordImage(PublicationRecord record) {
        byte[] body = record.canonicalTopologyBytes();
        return new Object[] {idImage(record.universe()), idImage(record.history()),
            idImage(record.owner()), idImage(record.system()), record.profile(), record.kind(),
            record.revision(), record.referencedPublicationRevision(),
            record.referencedTopologySha256().clone(), body == null ? null : body.clone()};
    }

    private static Object[] historyImage(SystemPublicationHistory history) {
        List<PublicationRecord> records = history.records();
        Object[][] rows = new Object[records.size()][];
        for (int i = 0; i < rows.length; i++) rows[i] = recordImage(records.get(i));
        return new Object[] {idImage(history.universe()), history.limits().keys(),
            history.limits().records(), history.limits().canonicalBytes(), history.keysLen(),
            history.canonicalBytesLen(), rows,
            recordImage(history.latest(id(0x22), id(0x33), id(0x44)))};
    }

    private static Object[] layoutImage(SystemPresentation layout) {
        return new Object[] {layout.profile(), idImage(layout.universe()), domainImage(layout.domain())};
    }

    private static Object[][] argumentRowsImage(SystemPlacementRegistry[] registries, Id[][] rows) {
        Object[][] image = new Object[rows.length][];
        for (int i = 0; i < rows.length; i++) {
            image[i] = new Object[] {registries[i] == null ? null : registryImage(registries[i]),
                idImage(rows[i][0]), idImage(rows[i][1]), idImage(rows[i][2])};
        }
        return image;
    }

    private static void unchangedRows(SystemPlacementRegistry heldRegistry,
            SystemPlacementRegistry[] registries, Id[][] rows, Object[][] before, Object[][] initial) {
        check(rows.length == 4 && registries.length == 4, "exact four nullable argument rows");
        for (int i = 0; i < 4; i++) {
            check(registries[i] == (NULL_REGISTRY[i] ? null : heldRegistry),
                    "fixed null registry position and actual held registry identity");
            check(rows[i].length == 3, "complete attempted key row");
            for (int j = 0; j < 3; j++) {
                String expected = QUERY_HEX[i][j];
                if (expected == null) {
                    check(rows[i][j] == null, "fixed null key sentinel");
                } else {
                    check(rows[i][j] != null && Arrays.equals(rows[i][j].value(), bytes(expected)),
                            "fixed independent nonnull raw key bytes");
                }
            }
        }
        Object[][] after = argumentRowsImage(registries, rows);
        check(Arrays.deepEquals(before, after) && Arrays.deepEquals(initial, after),
                "each refusal preserves all copied argument rows and null sentinels");
    }

    private static void fixedInputs(Fixture fixture) {
        String[] hex = {A1_HEX, A2_HEX}, sha = {A1_SHA, A2_SHA};
        for (int i = 0; i < 2; i++) {
            Topology input = fixture.topologies().get(i);
            check(Arrays.equals(fixture.originals()[i], bytes(hex[i]))
                    && fixture.originals()[i].length == 199, "complete original fixed bytes");
            check(input.universe().equals(id(0x11)) && input.history().equals(id(0x22))
                    && input.owner().equals(id(0x33)) && input.system().equals(id(0x44)), "fixed IDs");
            check(input.revision() == i + 1 && input.campaignDayMillis() == 2, "fixed source revisions/day");
            SystemPackageTopology.validate(input, fixture.limits());
            check(Arrays.equals(SystemPackageTopology.encode(input, fixture.limits()), bytes(hex[i])),
                    "maintained canonical bytes match independent literal");
            check(Arrays.equals(SystemPackageTopology.digest(input, fixture.limits()), bytes(sha[i])),
                    "maintained source digest matches independent SHA");
        }
    }

    private static void registryState(SystemPlacementRegistry registry) {
        check(registry.universe().equals(id(0x66)) && registry.capacity() == 2,
                "foreign empty registry namespace and capacity");
        check(registry.entries().isEmpty(), "foreign R0 has exactly zero entries");
    }

    private static void historyState(SystemPublicationHistory history, int count, int retainedBytes) {
        check(history.universe().equals(id(0x11)) && history.limits().keys() == 2
                && history.limits().records() == 3 && history.limits().canonicalBytes() == 597,
                "held history namespace and all limits");
        check(history.keysLen() == 1 && history.records().size() == count
                && history.canonicalBytesLen() == retainedBytes, "held history exact counters");
        String[] hex = {A1_HEX, A2_HEX}, hash = {A1_SHA, A2_SHA};
        for (int i = 0; i < count; i++) {
            PublicationRecord record = history.records().get(i);
            check(record.universe().equals(id(0x11)) && record.history().equals(id(0x22))
                    && record.owner().equals(id(0x33)) && record.system().equals(id(0x44)), "held record IDs");
            check(record.profile() == SystemPublicationHistory.Profile.SELECTED_TOPOLOGY_V1
                    && record.kind() == SystemPublicationHistory.Kind.PUBLISHED_VERSION, "held record profile/kind");
            check(record.revision() == i + 1 && record.referencedPublicationRevision() == i + 1,
                    "held event and reference revisions");
            check(Arrays.equals(record.referencedTopologySha256(), bytes(hash[i]))
                    && Arrays.equals(record.canonicalTopologyBytes(), bytes(hex[i])), "held fixed hash/body");
        }
        check(Arrays.deepEquals(recordImage(history.latest(id(0x22), id(0x33), id(0x44))),
                recordImage(history.records().get(count - 1))), "held latest full record");
    }

    private static void layoutState(SystemPresentation layout, Domain expected) {
        check(layout.profile() == Profile.GRID_256_V1 && layout.universe().equals(id(0x11)), "epoch identity");
        check(Arrays.deepEquals(domainImage(layout.domain()), domainImage(expected)), "all five epoch fields");
        check(layout.toString().equals("SystemPresentation{profile=GRID_256_V1}"), "safe exact layout text");
    }

    private static Object[] sourcesImage(Fixture fixture, SystemPlacementRegistry registry,
            SystemPublicationHistory history, SystemPresentation clientA, SystemPresentation clientB) {
        return new Object[] {inputImage(fixture), registryImage(registry), historyImage(history),
            layoutImage(clientA), layoutImage(clientB)};
    }

    private static void unchangedSources(Fixture fixture, SystemPlacementRegistry registry,
            SystemPublicationHistory history, SystemPresentation clientA, SystemPresentation clientB,
            Object[] before, Object[] initial, List<PublicationRecord> heldRecords,
            Object[] firstRecord, Object[] secondRecord) {
        registryState(registry);
        historyState(history, 2, 398);
        layoutState(clientA, fixture.domains().get(0));
        layoutState(clientB, fixture.domains().get(1));
        fixedInputs(fixture);
        check(fixture.limits().entities() == 128 && fixture.limits().dependencies() == 64
                && fixture.limits().labelBytes() == 128 && fixture.limits().documentBytes() == 65536,
                "all four fixed topology limits");
        check(fixture.universe().equals(id(0x11)) && fixture.query().get(0).equals(id(0x22))
                && fixture.query().get(1).equals(id(0x33)) && fixture.query().get(2).equals(id(0x44)),
                "all independent query and namespace IDs");
        Object[] after = sourcesImage(fixture, registry, history, clientA, clientB);
        check(Arrays.deepEquals(before, after) && Arrays.deepEquals(initial, after),
                "each refusal preserves complete copied source, epoch and input images");
        check(heldRecords.size() == 2 && Arrays.deepEquals(firstRecord, recordImage(heldRecords.get(0)))
                && Arrays.deepEquals(secondRecord, recordImage(heldRecords.get(1))),
                "retained history list and complete record values remain fixed");
    }

    private static void nullArgumentsRefuseBeforeForeignNamespaceWithoutMutatingClients() {
        byte[][] originals = {bytes(A1_HEX), bytes(A2_HEX)};
        Fixture fixture = new Fixture(id(0x11), List.of(id(0x22), id(0x33), id(0x44)),
                List.of(new Domain(-1024, -2048, 8192, 16384, 7),
                    new Domain(4096, -4096, 16384, 8192, 7)),
                List.of(SystemPackageTopology.decode(originals[0], SystemPackageTopology.DEFAULT),
                    SystemPackageTopology.decode(originals[1], SystemPackageTopology.DEFAULT)),
                originals, SystemPackageTopology.DEFAULT);
        fixedInputs(fixture);
        // This held History is only a nonmutation guard, never a project input or admission.
        SystemPublicationHistory history = new SystemPublicationHistory(fixture.universe(),
                new SystemPublicationHistory.Limits(2, 3, 597));
        check(history.records().isEmpty() && history.keysLen() == 0 && history.canonicalBytesLen() == 0,
                "empty held history before its two maintained fixture seeds");
        check(history.appendPublication(fixture.topologies().get(0), fixture.limits())
                == SystemPublicationHistory.Outcome.PUBLICATION_APPENDED, "held history A1 seed");
        historyState(history, 1, 199);
        check(history.appendPublication(fixture.topologies().get(1), fixture.limits())
                == SystemPublicationHistory.Outcome.PUBLICATION_APPENDED, "held history A2 seed");
        historyState(history, 2, 398);
        List<PublicationRecord> heldRecords = history.records();
        Object[] firstRecord = recordImage(heldRecords.get(0));
        Object[] secondRecord = recordImage(heldRecords.get(1));
        SystemPlacementRegistry registry = new SystemPlacementRegistry(id(0x66), 2);
        registryState(registry);
        SystemPresentation clientA = new SystemPresentation(fixture.universe(), fixture.domains().get(0));
        SystemPresentation clientB = new SystemPresentation(fixture.universe(), fixture.domains().get(1));
        layoutState(clientA, new Domain(-1024, -2048, 8192, 16384, 7));
        layoutState(clientB, new Domain(4096, -4096, 16384, 8192, 7));
        SystemPlacementRegistry[] registries = {null, registry, registry, registry};
        Id[][] rows = {
            {id(0x22), id(0x33), id(0x44)},
            {null, id(0x33), id(0x44)},
            {id(0x22), null, id(0x44)},
            {id(0x22), id(0x33), null}
        };
        Object[][] initialRows = argumentRowsImage(registries, rows);
        unchangedRows(registry, registries, rows, initialRows, initialRows);
        Object[] initial = sourcesImage(fixture, registry, history, clientA, clientB);
        unchangedSources(fixture, registry, history, clientA, clientB, initial, initial,
                heldRecords, firstRecord, secondRecord);

        Object[] beforeRegistry = sourcesImage(fixture, registry, history, clientA, clientB);
        Object[][] rowsBeforeRegistry = argumentRowsImage(registries, rows);
        try {
            clientA.project(registries[0], rows[0][0], rows[0][1], rows[0][2]);
            throw new AssertionError("null registry must refuse without an output");
        } catch (SystemPresentation.Failure failure) {
            check(failure.code() == SystemPresentation.ErrorCode.INVALID_INPUT,
                    "null registry exact typed invalid-input refusal");
            check(failure.getMessage().equals("INVALID_INPUT") && failure.getCause() == null,
                    "null registry exact public refusal message and null cause");
        }
        unchangedSources(fixture, registry, history, clientA, clientB, beforeRegistry, initial,
                heldRecords, firstRecord, secondRecord);
        unchangedRows(registry, registries, rows, rowsBeforeRegistry, initialRows);

        Object[] beforeHistory = sourcesImage(fixture, registry, history, clientA, clientB);
        Object[][] rowsBeforeHistory = argumentRowsImage(registries, rows);
        try {
            clientA.project(registries[1], rows[1][0], rows[1][1], rows[1][2]);
            throw new AssertionError("null history must refuse before foreign namespace without an output");
        } catch (SystemPresentation.Failure failure) {
            check(failure.code() == SystemPresentation.ErrorCode.INVALID_INPUT,
                    "null history exact typed invalid-input refusal");
            check(failure.getMessage().equals("INVALID_INPUT") && failure.getCause() == null,
                    "null history exact public refusal message and null cause");
        }
        unchangedSources(fixture, registry, history, clientA, clientB, beforeHistory, initial,
                heldRecords, firstRecord, secondRecord);
        unchangedRows(registry, registries, rows, rowsBeforeHistory, initialRows);

        Object[] beforeOwner = sourcesImage(fixture, registry, history, clientA, clientB);
        Object[][] rowsBeforeOwner = argumentRowsImage(registries, rows);
        try {
            clientA.project(registries[2], rows[2][0], rows[2][1], rows[2][2]);
            throw new AssertionError("null owner must refuse before foreign namespace without an output");
        } catch (SystemPresentation.Failure failure) {
            check(failure.code() == SystemPresentation.ErrorCode.INVALID_INPUT,
                    "null owner exact typed invalid-input refusal");
            check(failure.getMessage().equals("INVALID_INPUT") && failure.getCause() == null,
                    "null owner exact public refusal message and null cause");
        }
        unchangedSources(fixture, registry, history, clientA, clientB, beforeOwner, initial,
                heldRecords, firstRecord, secondRecord);
        unchangedRows(registry, registries, rows, rowsBeforeOwner, initialRows);

        Object[] beforeSystem = sourcesImage(fixture, registry, history, clientA, clientB);
        Object[][] rowsBeforeSystem = argumentRowsImage(registries, rows);
        try {
            clientA.project(registries[3], rows[3][0], rows[3][1], rows[3][2]);
            throw new AssertionError("null system must refuse before foreign namespace without an output");
        } catch (SystemPresentation.Failure failure) {
            check(failure.code() == SystemPresentation.ErrorCode.INVALID_INPUT,
                    "null system exact typed invalid-input refusal");
            check(failure.getMessage().equals("INVALID_INPUT") && failure.getCause() == null,
                    "null system exact public refusal message and null cause");
        }
        unchangedSources(fixture, registry, history, clientA, clientB, beforeSystem, initial,
                heldRecords, firstRecord, secondRecord);
        unchangedRows(registry, registries, rows, rowsBeforeSystem, initialRows);
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        nullArgumentsRefuseBeforeForeignNamespaceWithoutMutatingClients();
        System.out.println("Grid256V1 Java null arguments precede foreign namespace PASS; data only");
    }
}
