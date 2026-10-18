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

/** A real second reservation preserves seven owned client results; all values are data only. */
public final class SystemPresentationSecondReservationTest {
    private SystemPresentationSecondReservationTest() { }

    private static final String[] HEX = {"4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64", "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64", "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333555555555555555555555555555555550100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64"};
    private static final String[] SHA = {
        "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183",
        "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d",
        "6ea8fd3113fc299431367483e01f808086cdc28062ac9daf9d03a761fdcde821"
    };
    private static final long[] REVISIONS = {1, 2, 1};
    private static final int[] SYSTEMS = {0x44, 0x44, 0x55};
    private static final int[] OUTPUT_CATALOGUE = {0, 1, 2, 3, 4, 5, 2};
    private static final long[] OUTPUT_REVISIONS = {1, 1, 2, 2, 1, 1};
    private static final int[] OUTPUT_SYSTEMS = {0x44, 0x44, 0x44, 0x44, 0x55, 0x55};
    private static final int[] OUTPUT_SLOTS = {7, 7, 7, 7, 8, 8};
    private static final int[] OUTPUT_HASHES = {0, 0, 1, 1, 2, 2};
    private static final long[] OUTPUT_X = {-784, 4576, -784, 4576, -752, 4640};
    private static final long[] OUTPUT_Y = {-2016, -4080, -2016, -4080, -2016, -4080};

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
            check(t.dependencies().size() == 1, "fixed single dependency");
            Dependency dependency = t.dependencies().get(0);
            check(Arrays.equals(dependency.semanticKey().value(),
                    bytes("7777777777777777777777777777777777777777777777777777777777777777"))
                    && dependency.required(), "independent dependency key and required flag");
            check(t.entities().size() == 2, "fixed two entities");
            Entity star = t.entities().get(0), planet = t.entities().get(1);
            check(star.id().equals(id(0x01)) && star.kind() == SystemPackageTopology.Kind.STAR
                    && star.parent() == null && star.dependency() == null && "Sun".equals(star.label()),
                    "independent complete STAR fields");
            check(planet.id().equals(id(0x02)) && planet.kind() == SystemPackageTopology.Kind.PLANET
                    && planet.parent().equals(id(0x01)) && Integer.valueOf(0).equals(planet.dependency())
                    && "World".equals(planet.label()), "independent complete PLANET fields");
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

    private static void fixedRegistry(SystemPlacementRegistry registry, int stage) {
        check(registry.universe().equals(id(0x11)) && registry.capacity() == 2,
                "fixed registry namespace and capacity");
        int rows = stage == 0 ? 0 : stage == 3 ? 2 : 1;
        check(registry.entries().size() == rows, "complete independently fixed R0/R1/R2/R3 row count");
        for (int i = 0; i < rows; i++) {
            PlacementEntry e = registry.entries().get(i);
            int version = i == 0 ? (stage == 1 ? 1 : 2) : 1;
            int hash = i == 0 ? version - 1 : 2;
            check(e.history().equals(id(0x22)) && e.owner().equals(id(0x33))
                    && e.system().equals(id(i == 0 ? 0x44 : 0x55)), "full ordered registry key");
            check(e.slot().value() == (i == 0 ? 7 : 8) && e.revision() == version
                    && Arrays.equals(e.topologySha256(), bytes(SHA[hash])), "fixed row slot revision and hash");
        }
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
        int row = OUTPUT_CATALOGUE[index];
        check(output.profile() == Profile.GRID_256_V1 && output.universe().equals(id(0x11)),
                "fixed output profile and universe");
        check(output.history().equals(id(0x22)) && output.owner().equals(id(0x33))
                && output.system().equals(id(OUTPUT_SYSTEMS[row])), "fixed output full key");
        check(output.slot().value() == OUTPUT_SLOTS[row] && output.revision() == OUTPUT_REVISIONS[row]
                && Arrays.equals(output.topologySha256(), bytes(SHA[OUTPUT_HASHES[row]])),
                "fixed owned output slot revision and hash");
        check(Arrays.equals(domainImage(output.domain()), expectedDomain(row % 2))
                && output.x() == OUTPUT_X[row] && output.y() == OUTPUT_Y[row],
                "independent complete epoch and fixed coordinates");
    }

    private static void fixedOutputs(List<Held> outputs) {
        for (int i = 0; i < outputs.size(); i++) {
            Held h = outputs.get(i);
            int row = OUTPUT_CATALOGUE[i];
            fixedOutput(h.output(), i);
            check(Arrays.deepEquals(h.outputBefore(), outputImage(h.output()))
                    && Arrays.deepEquals(h.gettersBefore(),
                            getterImage(h.ids(), h.idBytes(), h.hash(), h.domain(), h.slot())),
                    "held output and every retained getter value remain unchanged");
            int[] ids = {0x11, 0x22, 0x33, OUTPUT_SYSTEMS[row]};
            for (int j = 0; j < 4; j++) {
                check(h.ids()[j].equals(id(ids[j]))
                        && Arrays.equals(h.idBytes()[j], id(ids[j]).value()), "fixed held IDs and arrays");
            }
            check(Arrays.equals(h.hash(), bytes(SHA[OUTPUT_HASHES[row]]))
                    && Arrays.equals(domainImage(h.domain()), expectedDomain(row % 2))
                    && h.slot().value() == OUTPUT_SLOTS[row], "fixed retained hash domain and slot");
        }
    }

    private static void defensiveCopies(ProjectedPlacement output) {
        Object[] before = outputImage(output);
        byte[] hash = output.topologySha256();
        hash[0] ^= 1;
        Id[] ids = {output.universe(), output.history(), output.owner(), output.system()};
        for (Id value : ids) {
            byte[] raw = value.value();
            byte[] ownedBefore = value.value().clone();
            raw[0] ^= 1;
            check(Arrays.equals(ownedBefore, value.value()), "returned Id owns defensive value bytes");
        }
        check(Arrays.deepEquals(before, outputImage(output)), "returned getter arrays cannot mutate result");
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

    private static void projectAndHold(Context c, SystemPresentation client, int index, int stage) {
        Object[] before = sourceImage(c);
        Object[][] oldOutputs = heldImages(c.outputs());
        Id[] query = index == 4 || index == 5 ? c.fixture().bKey() : c.fixture().aKey();
        ProjectedPlacement result = client.project(c.registry(), query[0], query[1], query[2]);
        fixedOutput(result, index);
        defensiveCopies(result);
        check(Arrays.deepEquals(oldOutputs, heldImages(c.outputs())), "all earlier outputs before retention");
        c.outputs().add(hold(result));
        unchanged(c, before, stage);
    }

    private static void nonoverlap(Context c) {
        for (int index : new int[] {0, 2, 6}) {
            ProjectedPlacement a = c.outputs().get(index).output(), b = c.outputs().get(4).output();
            check(Arrays.equals(domainImage(a.domain()), expectedDomain(0))
                    && Arrays.equals(domainImage(b.domain()), expectedDomain(0)), "same fixed client A epoch");
            check(a.x() - 7 == -791 && a.x() + 7 == -777 && a.y() - 7 == -2023
                    && a.y() + 7 == -2009 && b.x() - 7 == -759 && b.x() + 7 == -745
                    && b.y() - 7 == -2023 && b.y() + 7 == -2009, "fixed client A closed square endpoints");
            check(b.x() - a.x() == 32 && (b.x() - 7) - (a.x() + 7) == 18,
                    "client A center separation32 and disjoint endpoint gap18");
        }
        for (int index : new int[] {1, 3}) {
            ProjectedPlacement a = c.outputs().get(index).output(), b = c.outputs().get(5).output();
            check(Arrays.equals(domainImage(a.domain()), expectedDomain(1))
                    && Arrays.equals(domainImage(b.domain()), expectedDomain(1)), "same fixed client B epoch");
            check(a.x() - 7 == 4569 && a.x() + 7 == 4583 && a.y() - 7 == -4087
                    && a.y() + 7 == -4073 && b.x() - 7 == 4633 && b.x() + 7 == 4647
                    && b.y() - 7 == -4087 && b.y() + 7 == -4073, "fixed client B closed square endpoints");
            check(b.x() - a.x() == 64 && (b.x() - 7) - (a.x() + 7) == 50,
                    "client B center separation64 and disjoint endpoint gap50");
        }
    }

    private static void realSecondReservationPreservesClientOutputs() {
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
        fixedRegistry(registry, 0);
        Object[] beforeEmpty = sourceImage(c);
        Id[] aKey = f.aKey();
        int completedRefusals = 0;
        try {
            c.clientA().project(registry, aKey[0], aKey[1], aKey[2]);
            throw new AssertionError("R0 must have no projectable A placement");
        } catch (SystemPresentation.Failure failure) {
            check(failure.getClass() == SystemPresentation.Failure.class
                    && failure.code() == SystemPresentation.ErrorCode.MISSING_PLACEMENT
                    && failure.getMessage().equals("MISSING_PLACEMENT") && failure.getCause() == null,
                    "exact initial R0 missing placement refusal");
        }
        unchanged(c, beforeEmpty, 0);
        completedRefusals++;
        Object[] immutableBeforeInsert = immutableSources(c);
        Object[][] outputsBeforeInsert = heldImages(c.outputs());
        check(registry.reserve(topologies[0], f.slot(), f.limits())
                == SystemPlacementRegistry.ReservationOutcome.INSERTED, "actual A1 reservation");
        fixedRegistry(registry, 1);
        check(Arrays.deepEquals(immutableBeforeInsert, immutableSources(c))
                && Arrays.deepEquals(outputsBeforeInsert, heldImages(c.outputs())), "only R0 registry changes to R1");
        unchanged(c, sourceImage(c), 1);
        projectAndHold(c, c.clientA(), 0, 1);
        projectAndHold(c, c.clientB(), 1, 1);
        Object[] immutableBeforeUpdate = immutableSources(c);
        Object[][] outputsBeforeUpdate = heldImages(c.outputs());
        check(registry.reserve(topologies[1], f.slot(), f.limits())
                == SystemPlacementRegistry.ReservationOutcome.UPDATED, "actual A2 same-slot update");
        fixedOutputs(c.outputs()); // Before either A2 project, retain both original A1 outputs.
        fixedRegistry(registry, 2);
        check(Arrays.deepEquals(immutableBeforeUpdate, immutableSources(c))
                && Arrays.deepEquals(outputsBeforeUpdate, heldImages(c.outputs())), "only registry changes on update");
        unchanged(c, sourceImage(c), 2);
        projectAndHold(c, c.clientA(), 2, 2);
        projectAndHold(c, c.clientB(), 3, 2);
        check(c.outputs().size() == 4, "four actual fixed earlier outputs");

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
        check(completedRefusals == 3 && c.outputs().size() == 4, "three exact refusals and no B output yet");

        Object[] immutableBeforeSecond = immutableSources(c);
        Object[][] outputsBeforeSecond = heldImages(c.outputs());
        check(registry.reserve(topologies[2], new ConceptualSlot(8), f.limits())
                == SystemPlacementRegistry.ReservationOutcome.INSERTED, "actual noncolliding B1 reservation");
        fixedRegistry(registry, 3);
        fixedOutputs(c.outputs()); // Old A1/A2 outputs must survive before any B projection.
        check(Arrays.deepEquals(immutableBeforeSecond, immutableSources(c))
                && Arrays.deepEquals(outputsBeforeSecond, heldImages(c.outputs())), "only R2 registry changes to R3");
        unchanged(c, sourceImage(c), 3);
        projectAndHold(c, c.clientA(), 4, 3);
        projectAndHold(c, c.clientB(), 5, 3);
        check(c.outputs().size() == 6, "six owned catalogue outputs before A reprojection");
        Object[][] sixBeforeRepeat = heldImages(c.outputs());
        projectAndHold(c, c.clientA(), 6, 3); // Reproject A2, never reserve A2 again.
        check(Arrays.deepEquals(sixBeforeRepeat, heldImages(c.outputs().subList(0, 6))),
                "all six earlier output and getter images survive A reprojection");
        check(c.outputs().size() == 7 && completedRefusals == 3, "seven actual positive projects, three refusals");
        nonoverlap(c);
        unchanged(c, sourceImage(c), 3);
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        realSecondReservationPreservesClientOutputs();
        System.out.println("Grid256V1 Java second reservation preserves client outputs PASS; data only");
    }
}
