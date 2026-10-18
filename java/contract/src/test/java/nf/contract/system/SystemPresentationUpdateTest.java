package nf.contract.system;

import java.util.Arrays;
import java.util.HexFormat;
import java.util.List;
import nf.contract.system.SystemPackageTopology.Dependency;
import nf.contract.system.SystemPackageTopology.Entity;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPackageTopology.Topology;
import nf.contract.system.SystemPlacementRegistry.ConceptualSlot;
import nf.contract.system.SystemPlacementRegistry.PlacementEntry;
import nf.contract.system.SystemPresentation.Domain;
import nf.contract.system.SystemPresentation.ProjectedPlacement;
import nf.contract.system.SystemPresentation.Profile;
import nf.contract.system.SystemPublicationHistory.PublicationRecord;

/** A real source update preserves client positions and earlier owned results; data only. */
public final class SystemPresentationUpdateTest {
    private SystemPresentationUpdateTest() { }

    private static final String A1_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A2_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A1_SHA = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
    private static final String A2_SHA = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";

    private record Fixture(Id universe, List<Id> query, ConceptualSlot slot, List<Domain> domains,
            List<Topology> topologies, byte[][] originals, SystemPackageTopology.Limits limits) { }

    private record ExpectedVersion(long revision, String sha) { }

    private static final ExpectedVersion VERSION_ONE = new ExpectedVersion(1, A1_SHA);
    private static final ExpectedVersion VERSION_TWO = new ExpectedVersion(2, A2_SHA);

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
            idImage(fixture.query().get(1)), idImage(fixture.query().get(2)), fixture.slot().value(),
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

    private static Object[] projectedImage(ProjectedPlacement value) {
        return new Object[] {value.profile(), idImage(value.universe()), idImage(value.history()),
            idImage(value.owner()), idImage(value.system()), value.slot().value(), value.revision(),
            value.topologySha256().clone(), domainImage(value.domain()), value.x(), value.y()};
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
            check(Arrays.equals(SystemPackageTopology.encode(input, fixture.limits()), bytes(hex[i])),
                    "maintained canonical bytes match independent literal");
            check(Arrays.equals(SystemPackageTopology.digest(input, fixture.limits()), bytes(sha[i])),
                    "maintained source digest matches independent SHA");
        }
    }

    private static void registryState(SystemPlacementRegistry registry, ExpectedVersion version) {
        check(registry.universe().equals(id(0x11)) && registry.capacity() == 2, "registry metadata");
        check(registry.entries().size() == 1, "exact one-row order");
        PlacementEntry entry = registry.entries().get(0);
        check(entry.history().equals(id(0x22)) && entry.owner().equals(id(0x33))
                && entry.system().equals(id(0x44)), "complete registry full key");
        check(entry.slot().value() == 7 && entry.revision() == version.revision(), "registry stable slot and expected revision");
        check(Arrays.equals(entry.topologySha256(), bytes(version.sha())), "registry independent expected source SHA");
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

    private static void projected(ProjectedPlacement actual, Domain expected,
            ExpectedVersion version, long x, long y) {
        check(actual.profile() == Profile.GRID_256_V1 && actual.universe().equals(id(0x11)), "output profile/namespace");
        check(actual.history().equals(id(0x22)) && actual.owner().equals(id(0x33))
                && actual.system().equals(id(0x44)), "output full key");
        check(actual.slot().value() == 7 && actual.revision() == version.revision(), "output source slot/revision");
        check(Arrays.equals(actual.topologySha256(), bytes(version.sha())), "output independent fixed hash");
        check(Arrays.deepEquals(domainImage(actual.domain()), domainImage(expected)), "output whole domain");
        check(actual.x() == x && actual.y() == y, "independent fixed coordinates");
        check(actual.toString().equals("ProjectedPlacement{profile=GRID_256_V1}"), "safe exact output text");
    }

    private static Object[] sourcesImage(Fixture fixture, SystemPlacementRegistry registry,
            SystemPublicationHistory history, SystemPresentation clientA, SystemPresentation clientB) {
        return new Object[] {inputImage(fixture), registryImage(registry), historyImage(history),
            layoutImage(clientA), layoutImage(clientB)};
    }

    private static void unchangedSources(Fixture fixture, SystemPlacementRegistry registry,
            SystemPublicationHistory history, SystemPresentation clientA, SystemPresentation clientB,
            Object[] before, ExpectedVersion version) {
        registryState(registry, version);
        historyState(history, 2, 398);
        layoutState(clientA, fixture.domains().get(0));
        layoutState(clientB, fixture.domains().get(1));
        fixedInputs(fixture);
        check(Arrays.deepEquals(before, sourcesImage(fixture, registry, history, clientA, clientB)),
                "each projection preserves every source, epoch and input field");
    }

    private static void realUpdatePreservesEachClientPositionAndEarlierSourceBinding() {
        byte[][] originals = {bytes(A1_HEX), bytes(A2_HEX)};
        Fixture fixture = new Fixture(id(0x11), List.of(id(0x22), id(0x33), id(0x44)),
                new ConceptualSlot(7), List.of(new Domain(-1024, -2048, 8192, 16384, 7),
                    new Domain(4096, -4096, 16384, 8192, 7)),
                List.of(SystemPackageTopology.decode(originals[0], SystemPackageTopology.DEFAULT),
                    SystemPackageTopology.decode(originals[1], SystemPackageTopology.DEFAULT)),
                originals, SystemPackageTopology.DEFAULT);
        fixedInputs(fixture);
        check(fixture.limits().entities() == 128 && fixture.limits().dependencies() == 64
                && fixture.limits().labelBytes() == 128 && fixture.limits().documentBytes() == 65536,
                "all four actual default topology limits");
        Object[] inputsBefore = inputImage(fixture);
        // This held History is only a nonmutation guard, never projection admission.
        SystemPublicationHistory history = new SystemPublicationHistory(fixture.universe(),
                new SystemPublicationHistory.Limits(2, 3, 597));
        check(history.records().isEmpty() && history.keysLen() == 0 && history.canonicalBytesLen() == 0,
                "empty held history");
        check(history.appendPublication(fixture.topologies().get(0), fixture.limits())
                == SystemPublicationHistory.Outcome.PUBLICATION_APPENDED, "held history A1 seed");
        historyState(history, 1, 199);
        check(history.appendPublication(fixture.topologies().get(1), fixture.limits())
                == SystemPublicationHistory.Outcome.PUBLICATION_APPENDED, "held history A2 seed");
        historyState(history, 2, 398);
        Object[] historyBefore = historyImage(history);
        List<PublicationRecord> heldRecords = history.records();
        Object[] heldFirst = recordImage(heldRecords.get(0)), heldSecond = recordImage(heldRecords.get(1));
        SystemPlacementRegistry registry = new SystemPlacementRegistry(fixture.universe(), 2);
        check(registry.universe().equals(id(0x11)) && registry.capacity() == 2
                && registry.entries().isEmpty(), "empty source registry");
        check(registry.reserve(fixture.topologies().get(0), fixture.slot(), fixture.limits())
                == SystemPlacementRegistry.ReservationOutcome.INSERTED, "actual A1 reservation");
        registryState(registry, VERSION_ONE);
        SystemPresentation clientA = new SystemPresentation(fixture.universe(), fixture.domains().get(0));
        SystemPresentation clientB = new SystemPresentation(fixture.universe(), fixture.domains().get(1));
        layoutState(clientA, fixture.domains().get(0));
        layoutState(clientB, fixture.domains().get(1));
        Object[] layoutA = layoutImage(clientA), layoutB = layoutImage(clientB);
        List<Id> key = fixture.query();
        Object[] r1Before = sourcesImage(fixture, registry, history, clientA, clientB);

        ProjectedPlacement oldA = clientA.project(registry, key.get(0), key.get(1), key.get(2));
        projected(oldA, fixture.domains().get(0), VERSION_ONE, -784, -2016);
        unchangedSources(fixture, registry, history, clientA, clientB, r1Before, VERSION_ONE);
        Object[] oldAImage = projectedImage(oldA);
        byte[] oldAHash = oldA.topologySha256();
        Id oldAId = oldA.system();
        byte[] oldAIdBytes = oldAId.value();
        ProjectedPlacement oldB = clientB.project(registry, key.get(0), key.get(1), key.get(2));
        projected(oldB, fixture.domains().get(1), VERSION_ONE, 4576, -4080);
        unchangedSources(fixture, registry, history, clientA, clientB, r1Before, VERSION_ONE);
        check(Arrays.deepEquals(oldAImage, projectedImage(oldA)), "first A1 result survives second call");
        Object[] oldBImage = projectedImage(oldB);
        byte[] oldBHash = oldB.topologySha256();
        Id oldBId = oldB.system();
        byte[] oldBIdBytes = oldBId.value();
        check(Arrays.deepEquals(inputsBefore, inputImage(fixture)), "all setup inputs remain fixed");

        check(registry.reserve(fixture.topologies().get(1), fixture.slot(), fixture.limits())
                == SystemPlacementRegistry.ReservationOutcome.UPDATED, "actual A2 update at same slot");
        registryState(registry, VERSION_TWO);
        // Check actual held A1 objects immediately after update, BEFORE either A2 projection.
        projected(oldA, fixture.domains().get(0), VERSION_ONE, -784, -2016);
        projected(oldB, fixture.domains().get(1), VERSION_ONE, 4576, -4080);
        check(Arrays.deepEquals(oldAImage, projectedImage(oldA))
                && Arrays.deepEquals(oldBImage, projectedImage(oldB)), "update preserves old source bindings");
        check(Arrays.equals(oldAHash, bytes(A1_SHA)) && Arrays.equals(oldBHash, bytes(A1_SHA))
                && oldAId.equals(id(0x44)) && oldBId.equals(id(0x44))
                && Arrays.equals(oldAIdBytes, id(0x44).value())
                && Arrays.equals(oldBIdBytes, id(0x44).value()), "update preserves retained getter values");
        check(Arrays.deepEquals(inputsBefore, inputImage(fixture))
                && Arrays.deepEquals(historyBefore, historyImage(history))
                && Arrays.deepEquals(layoutA, layoutImage(clientA))
                && Arrays.deepEquals(layoutB, layoutImage(clientB)), "update changes only source registry");
        fixedInputs(fixture);
        historyState(history, 2, 398);
        Object[] r2Before = sourcesImage(fixture, registry, history, clientA, clientB);

        ProjectedPlacement newA = clientA.project(registry, key.get(0), key.get(1), key.get(2));
        projected(newA, fixture.domains().get(0), VERSION_TWO, -784, -2016);
        unchangedSources(fixture, registry, history, clientA, clientB, r2Before, VERSION_TWO);
        check(Arrays.deepEquals(oldAImage, projectedImage(oldA))
                && Arrays.deepEquals(oldBImage, projectedImage(oldB)), "first A2 call preserves old results");
        Object[] newAImage = projectedImage(newA);
        ProjectedPlacement newB = clientB.project(registry, key.get(0), key.get(1), key.get(2));
        projected(newB, fixture.domains().get(1), VERSION_TWO, 4576, -4080);
        unchangedSources(fixture, registry, history, clientA, clientB, r2Before, VERSION_TWO);
        check(Arrays.deepEquals(oldAImage, projectedImage(oldA))
                && Arrays.deepEquals(oldBImage, projectedImage(oldB))
                && Arrays.deepEquals(newAImage, projectedImage(newA)), "second A2 call preserves earlier results");
        Object[] newBImage = projectedImage(newB);
        check(oldA.x() == newA.x() && oldA.y() == newA.y()
                && oldB.x() == newB.x() && oldB.y() == newB.y(), "each client retains its fixed position");
        check(!oldA.equals(newA) && !oldB.equals(newB), "new source binding differs from held A1");

        for (ProjectedPlacement result : List.of(oldA, oldB, newA, newB)) {
            for (Id returned : List.of(result.universe(), result.history(), result.owner(), result.system())) {
                Arrays.fill(returned.value(), (byte) 0);
            }
            Arrays.fill(result.topologySha256(), (byte) 0);
        }
        Arrays.fill(clientA.universe().value(), (byte) 0);
        Arrays.fill(clientB.universe().value(), (byte) 0);
        projected(oldA, fixture.domains().get(0), VERSION_ONE, -784, -2016);
        projected(oldB, fixture.domains().get(1), VERSION_ONE, 4576, -4080);
        projected(newA, fixture.domains().get(0), VERSION_TWO, -784, -2016);
        projected(newB, fixture.domains().get(1), VERSION_TWO, 4576, -4080);
        check(Arrays.deepEquals(oldAImage, projectedImage(oldA))
                && Arrays.deepEquals(oldBImage, projectedImage(oldB))
                && Arrays.deepEquals(newAImage, projectedImage(newA))
                && Arrays.deepEquals(newBImage, projectedImage(newB)), "all four output values remain owned");
        check(Arrays.equals(oldAHash, bytes(A1_SHA)) && Arrays.equals(oldBHash, bytes(A1_SHA))
                && oldAId.equals(id(0x44)) && oldBId.equals(id(0x44))
                && Arrays.equals(oldAIdBytes, id(0x44).value())
                && Arrays.equals(oldBIdBytes, id(0x44).value()), "old returned values remain unchanged");
        unchangedSources(fixture, registry, history, clientA, clientB, r2Before, VERSION_TWO);
        check(Arrays.deepEquals(historyBefore, historyImage(history))
                && heldRecords.size() == 2 && Arrays.deepEquals(heldFirst, recordImage(heldRecords.get(0)))
                && Arrays.deepEquals(heldSecond, recordImage(heldRecords.get(1))), "held history list and records unchanged");
        check(Arrays.deepEquals(inputsBefore, inputImage(fixture))
                && Arrays.deepEquals(layoutA, layoutImage(clientA))
                && Arrays.deepEquals(layoutB, layoutImage(clientB)), "all inputs and both epochs unchanged");
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        realUpdatePreservesEachClientPositionAndEarlierSourceBinding();
        System.out.println("Grid256V1 Java reservation update retains earlier results PASS; data only");
    }
}
