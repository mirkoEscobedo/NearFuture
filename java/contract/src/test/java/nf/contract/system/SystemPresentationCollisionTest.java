package nf.contract.system;

import java.util.ArrayList;
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

/** A real occupied-slot refusal creates no projected B placement; all values are data only. */
public final class SystemPresentationCollisionTest {
    private SystemPresentationCollisionTest() { }

    private static final String[] HEX = {"4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64", "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64", "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333555555555555555555555555555555550100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64"};
    private static final String[] SHA = {
        "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183",
        "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d",
        "6ea8fd3113fc299431367483e01f808086cdc28062ac9daf9d03a761fdcde821"
    };
    private static final long[] REVISIONS = {1, 2, 1};
    private static final int[] SYSTEMS = {0x44, 0x44, 0x55};
    private static final long[] OUTPUT_REVISIONS = {1, 1, 2, 2};
    private static final long[] OUTPUT_X = {-784, 4576, -784, 4576};
    private static final long[] OUTPUT_Y = {-2016, -4080, -2016, -4080};

    private record Fixture(Id universe, Id[] aKey, Id[] bKey, Domain[] domains,
            Topology[] topologies, byte[][] originals, ConceptualSlot slot,
            SystemPackageTopology.Limits limits) { }

    private record Held(ProjectedPlacement output, Id[] ids, byte[][] idBytes, byte[] hash,
            Domain domain, ConceptualSlot slot, Object[] outputBefore, Object[] gettersBefore) { }

    private record Context(Fixture fixture, SystemPlacementRegistry registry,
            SystemPublicationHistory history, List<PublicationRecord> records,
            SystemPresentation clientA, SystemPresentation clientB, List<Held> outputs) { }

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

    private static Object[] keyImage(Id[] key) {
        Object[] image = new Object[key.length];
        for (int i = 0; i < key.length; i++) image[i] = idImage(key[i]);
        return image;
    }

    private static long[] domainImage(Domain domain) {
        return new long[] {domain.originX(), domain.originY(), domain.width(),
                domain.height(), domain.halfExtent()};
    }

    private static long[] expectedDomain(int client) {
        return client == 0 ? new long[] {-1024, -2048, 8192, 16384, 7}
                : new long[] {4096, -4096, 16384, 8192, 7};
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
        return new Object[] {idImage(value.universe()), idImage(value.history()),
                idImage(value.owner()), idImage(value.system()), value.revision(),
                value.campaignDayMillis(), dependencies, entities};
    }

    private static Object[] inputImage(Fixture f) {
        Object[][] topologies = new Object[3][];
        byte[][] originals = new byte[3][];
        for (int i = 0; i < 3; i++) {
            topologies[i] = topologyImage(f.topologies()[i]);
            originals[i] = f.originals()[i].clone();
        }
        return new Object[] {idImage(f.universe()), keyImage(f.aKey()), keyImage(f.bKey()),
                domainImage(f.domains()[0]), domainImage(f.domains()[1]), topologies, originals,
                f.slot().value(), f.limits().entities(), f.limits().dependencies(),
                f.limits().labelBytes(), f.limits().documentBytes()};
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

    private static Object[][] recordImages(List<PublicationRecord> records) {
        Object[][] rows = new Object[records.size()][];
        for (int i = 0; i < rows.length; i++) rows[i] = recordImage(records.get(i));
        return rows;
    }

    private static Object[] historyImage(SystemPublicationHistory history) {
        return new Object[] {idImage(history.universe()), history.limits().keys(),
                history.limits().records(), history.limits().canonicalBytes(), history.keysLen(),
                history.canonicalBytesLen(), recordImages(history.records()),
                recordImage(history.latest(id(0x22), id(0x33), id(0x44)))};
    }

    private static Object[] layoutImage(SystemPresentation layout) {
        return new Object[] {layout.profile(), idImage(layout.universe()), domainImage(layout.domain())};
    }

    private static Object[] outputImage(ProjectedPlacement output) {
        return new Object[] {output.profile(), idImage(output.universe()), idImage(output.history()),
                idImage(output.owner()), idImage(output.system()), output.slot().value(),
                output.revision(), output.topologySha256().clone(), domainImage(output.domain()),
                output.x(), output.y()};
    }

    private static Object[] getterImage(Id[] ids, byte[][] rawIds, byte[] hash,
            Domain domain, ConceptualSlot slot) {
        byte[][] copied = new byte[rawIds.length][];
        for (int i = 0; i < copied.length; i++) copied[i] = rawIds[i].clone();
        return new Object[] {keyImage(ids), copied, hash.clone(), domainImage(domain), slot.value()};
    }

    private static Held hold(ProjectedPlacement output) {
        Id[] ids = {output.universe(), output.history(), output.owner(), output.system()};
        byte[][] rawIds = new byte[4][];
        for (int i = 0; i < 4; i++) rawIds[i] = ids[i].value();
        byte[] hash = output.topologySha256();
        Domain domain = output.domain();
        ConceptualSlot slot = output.slot();
        return new Held(output, ids, rawIds, hash, domain, slot, outputImage(output),
                getterImage(ids, rawIds, hash, domain, slot));
    }

    private static Object[][] heldImages(List<Held> outputs) {
        Object[][] images = new Object[outputs.size()][];
        for (int i = 0; i < images.length; i++) {
            Held h = outputs.get(i);
            images[i] = new Object[] {outputImage(h.output()),
                    getterImage(h.ids(), h.idBytes(), h.hash(), h.domain(), h.slot())};
        }
        return images;
    }

    private static void fixedInputs(Fixture f) {
        check(f.universe().equals(id(0x11)) && f.slot().value() == 7, "fixed namespace and occupied slot");
        check(f.aKey().length == 3 && f.bKey().length == 3, "complete A and B keys");
        for (int i = 0; i < 3; i++) {
            check(f.aKey()[i].equals(id(new int[] {0x22, 0x33, 0x44}[i]))
                    && f.bKey()[i].equals(id(new int[] {0x22, 0x33, 0x55}[i])), "fixed query IDs");
            Topology t = f.topologies()[i];
            check(Arrays.equals(f.originals()[i], bytes(HEX[i])) && f.originals()[i].length == 199,
                    "independent complete original body");
            check(t.universe().equals(id(0x11)) && t.history().equals(id(0x22))
                    && t.owner().equals(id(0x33)) && t.system().equals(id(SYSTEMS[i])),
                    "fixed topology full identity");
            check(t.revision() == REVISIONS[i] && t.campaignDayMillis() == 2, "fixed revision and day");
            SystemPackageTopology.validate(t, f.limits());
            check(Arrays.equals(SystemPackageTopology.encode(t, f.limits()), bytes(HEX[i]))
                    && Arrays.equals(SystemPackageTopology.digest(t, f.limits()), bytes(SHA[i])),
                    "actual canonical bytes and digest equal independent constants");
        }
        check(f.limits().entities() == 128 && f.limits().dependencies() == 64
                && f.limits().labelBytes() == 128 && f.limits().documentBytes() == 65536,
                "all four default topology limits");
        for (int i = 0; i < 2; i++) {
            check(Arrays.equals(domainImage(f.domains()[i]), expectedDomain(i)), "fixed all-five domain");
        }
    }

    private static void fixedRegistry(SystemPlacementRegistry registry, int version) {
        check(registry.universe().equals(id(0x11)) && registry.capacity() == 2,
                "populated registry namespace and independent capacity");
        check(registry.entries().size() == 1, "one exact A row, never empty or full capacity");
        PlacementEntry e = registry.entries().get(0);
        check(e.history().equals(id(0x22)) && e.owner().equals(id(0x33))
                && e.system().equals(id(0x44)), "full A registry key");
        check(e.slot().value() == 7 && e.revision() == version
                && Arrays.equals(e.topologySha256(), bytes(SHA[version - 1])), "fixed row source metadata");
    }

    private static void fixedRecord(PublicationRecord r, int version) {
        check(r.universe().equals(id(0x11)) && r.history().equals(id(0x22))
                && r.owner().equals(id(0x33)) && r.system().equals(id(0x44)), "all four held record IDs");
        check(r.profile() == SystemPublicationHistory.Profile.SELECTED_TOPOLOGY_V1
                && r.kind() == SystemPublicationHistory.Kind.PUBLISHED_VERSION, "record profile and kind");
        check(r.revision() == version && r.referencedPublicationRevision() == version,
                "event and publication-reference revisions");
        check(Arrays.equals(r.referencedTopologySha256(), bytes(SHA[version - 1]))
                && Arrays.equals(r.canonicalTopologyBytes(), bytes(HEX[version - 1])),
                "independently fixed full record hash and body");
    }

    private static void fixedHistory(SystemPublicationHistory history, int count) {
        check(history.universe().equals(id(0x11)) && history.limits().keys() == 2
                && history.limits().records() == 3 && history.limits().canonicalBytes() == 597,
                "held History namespace and all three limits");
        check(history.keysLen() == 1 && history.records().size() == count
                && history.canonicalBytesLen() == (count == 1 ? 199 : 398), "exact held counters");
        for (int i = 0; i < count; i++) fixedRecord(history.records().get(i), i + 1);
        fixedRecord(history.latest(id(0x22), id(0x33), id(0x44)), count);
    }

    private static void fixedOutput(ProjectedPlacement output, int index) {
        int version = (int) OUTPUT_REVISIONS[index];
        check(output.profile() == Profile.GRID_256_V1 && output.universe().equals(id(0x11)),
                "fixed output profile and universe");
        check(output.history().equals(id(0x22)) && output.owner().equals(id(0x33))
                && output.system().equals(id(0x44)), "fixed output full key");
        check(output.slot().value() == 7 && output.revision() == version
                && Arrays.equals(output.topologySha256(), bytes(SHA[version - 1])),
                "fixed owned output slot revision and hash");
        check(Arrays.equals(domainImage(output.domain()), expectedDomain(index % 2))
                && output.x() == OUTPUT_X[index] && output.y() == OUTPUT_Y[index],
                "independent complete epoch and fixed coordinates");
    }

    private static void fixedOutputs(List<Held> outputs) {
        for (int i = 0; i < outputs.size(); i++) {
            Held h = outputs.get(i);
            fixedOutput(h.output(), i);
            check(Arrays.deepEquals(h.outputBefore(), outputImage(h.output()))
                    && Arrays.deepEquals(h.gettersBefore(),
                            getterImage(h.ids(), h.idBytes(), h.hash(), h.domain(), h.slot())),
                    "held output and every retained getter value remain unchanged");
            int[] ids = {0x11, 0x22, 0x33, 0x44};
            for (int j = 0; j < 4; j++) {
                check(h.ids()[j].equals(id(ids[j]))
                        && Arrays.equals(h.idBytes()[j], id(ids[j]).value()), "fixed held IDs and arrays");
            }
            check(Arrays.equals(h.hash(), bytes(SHA[(int) OUTPUT_REVISIONS[i] - 1]))
                    && Arrays.equals(domainImage(h.domain()), expectedDomain(i % 2))
                    && h.slot().value() == 7, "fixed retained hash domain and slot");
        }
    }

    private static Object[] immutableSources(Context c) {
        return new Object[] {inputImage(c.fixture()), historyImage(c.history()),
                recordImages(c.records()), layoutImage(c.clientA()), layoutImage(c.clientB())};
    }

    private static Object[] sourceImage(Context c) {
        return new Object[] {registryImage(c.registry()), immutableSources(c)};
    }

    private static void unchanged(Context c, Object[] before, int version) {
        fixedInputs(c.fixture());
        fixedRegistry(c.registry(), version);
        fixedHistory(c.history(), 2);
        check(c.records().size() == 2, "retained History list length");
        fixedRecord(c.records().get(0), 1);
        fixedRecord(c.records().get(1), 2);
        SystemPresentation[] layouts = {c.clientA(), c.clientB()};
        for (int i = 0; i < 2; i++) {
            check(layouts[i].profile() == Profile.GRID_256_V1 && layouts[i].universe().equals(id(0x11))
                    && Arrays.equals(domainImage(layouts[i].domain()), expectedDomain(i)),
                    "complete fixed epoch identity");
        }
        check(Arrays.deepEquals(before, sourceImage(c)), "every copied source field unchanged");
        fixedOutputs(c.outputs());
    }

    private static void projectAndHold(Context c, SystemPresentation client, int index, int version) {
        Object[] before = sourceImage(c);
        Id[] query = c.fixture().aKey();
        ProjectedPlacement result = client.project(c.registry(), query[0], query[1], query[2]);
        fixedOutput(result, index);
        c.outputs().add(hold(result));
        unchanged(c, before, version);
    }

    private static void realCollisionLeavesNoProjectablePlacement() {
        byte[][] originals = {bytes(HEX[0]), bytes(HEX[1]), bytes(HEX[2])};
        Topology[] topologies = new Topology[3];
        for (int i = 0; i < 3; i++) topologies[i] = SystemPackageTopology.decode(originals[i], SystemPackageTopology.DEFAULT);
        Fixture f = new Fixture(id(0x11), new Id[] {id(0x22), id(0x33), id(0x44)},
                new Id[] {id(0x22), id(0x33), id(0x55)},
                new Domain[] {new Domain(-1024, -2048, 8192, 16384, 7),
                    new Domain(4096, -4096, 16384, 8192, 7)},
                topologies, originals, new ConceptualSlot(7), SystemPackageTopology.DEFAULT);
        fixedInputs(f);
        Object[] inputsBefore = inputImage(f);
        // The independently seeded History is only a guard, never projection admission.
        SystemPublicationHistory history = new SystemPublicationHistory(f.universe(),
                new SystemPublicationHistory.Limits(2, 3, 597));
        check(history.appendPublication(topologies[0], f.limits())
                == SystemPublicationHistory.Outcome.PUBLICATION_APPENDED, "actual held A1 seed");
        fixedHistory(history, 1);
        check(history.appendPublication(topologies[1], f.limits())
                == SystemPublicationHistory.Outcome.PUBLICATION_APPENDED, "actual held A2 seed");
        fixedHistory(history, 2);
        SystemPlacementRegistry registry = new SystemPlacementRegistry(f.universe(), 2);
        Context c = new Context(f, registry, history, history.records(),
                new SystemPresentation(f.universe(), f.domains()[0]),
                new SystemPresentation(f.universe(), f.domains()[1]), new ArrayList<>());
        check(Arrays.deepEquals(inputsBefore, inputImage(f)), "fixture inputs unchanged by setup");
        check(registry.reserve(topologies[0], f.slot(), f.limits())
                == SystemPlacementRegistry.ReservationOutcome.INSERTED, "actual A1 reservation");
        fixedRegistry(registry, 1);
        projectAndHold(c, c.clientA(), 0, 1);
        projectAndHold(c, c.clientB(), 1, 1);
        Object[] immutableBeforeUpdate = immutableSources(c);
        check(registry.reserve(topologies[1], f.slot(), f.limits())
                == SystemPlacementRegistry.ReservationOutcome.UPDATED, "actual A2 same-slot update");
        fixedOutputs(c.outputs()); // Before either A2 project, retain both original A1 outputs.
        fixedRegistry(registry, 2);
        check(Arrays.deepEquals(immutableBeforeUpdate, immutableSources(c)), "only registry changes on update");
        projectAndHold(c, c.clientA(), 2, 2);
        projectAndHold(c, c.clientB(), 3, 2);
        check(c.outputs().size() == 4, "four actual fixed earlier outputs");
        int completedRefusals = 0;

        Object[] beforeCollision = sourceImage(c);
        Object[][] outputsBeforeCollision = heldImages(c.outputs());
        try {
            registry.reserve(topologies[2], f.slot(), f.limits());
            throw new AssertionError("actual B1 occupied-slot reservation must refuse");
        } catch (SystemPlacementRegistry.Failure failure) {
            check(failure.getClass() == SystemPlacementRegistry.Failure.class
                    && failure.code() == SystemPlacementRegistry.PlacementError.SLOT_OCCUPIED,
                    "exact existing reservation refusal, not presentation failure");
            check(failure.getMessage().equals("SLOT_OCCUPIED") && failure.getCause() == null
                    && failure.topologyCode() == null, "exact collision message and null cause/topology");
        }
        unchanged(c, beforeCollision, 2);
        check(Arrays.deepEquals(outputsBeforeCollision, heldImages(c.outputs())), "all four held getter images after collision");
        completedRefusals++;

        Object[] beforeMissing = sourceImage(c);
        Object[][] outputsBeforeMissing = heldImages(c.outputs());
        Id[] bKey = f.bKey();
        try {
            c.clientA().project(registry, bKey[0], bKey[1], bKey[2]);
            throw new AssertionError("refused B1 must have no projectable placement in populated R2");
        } catch (SystemPresentation.Failure failure) {
            check(failure.getClass() == SystemPresentation.Failure.class
                    && failure.code() == SystemPresentation.ErrorCode.MISSING_PLACEMENT,
                    "exact typed missing B projection, not a collision output");
            check(failure.getMessage().equals("MISSING_PLACEMENT") && failure.getCause() == null,
                    "exact missing placement message and null cause");
        }
        unchanged(c, beforeMissing, 2);
        check(Arrays.deepEquals(outputsBeforeMissing, heldImages(c.outputs())), "all four held getter images after missing B");
        completedRefusals++;
        check(completedRefusals == 2 && c.outputs().size() == 4, "complete composed refusal case, no B output");
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        realCollisionLeavesNoProjectablePlacement();
        System.out.println("Grid256V1 Java real collision leaves no projectable placement PASS; data only");
    }
}
