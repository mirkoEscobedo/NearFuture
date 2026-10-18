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

/** Real unsigned slot extrema preserve owned client results and complete sources; data only. */
public final class SystemPresentationUnsignedSlotsTest {
    private SystemPresentationUnsignedSlotsTest() { }

    private static final String[] HEX = {
        "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
        "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64"
    };
    private static final String[] SHA = {
        "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183",
        "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d"
    };
    private static final int[] SLOTS = {0, 65535, 0, 65535};
    private static final int[] CLIENTS = {0, 0, 1, 1};
    private static final long[] X = {-1008, 7152, 4128, 20448};
    private static final long[] Y = {-2016, 14304, -4080, 4080};

    private record Fixture(Id universe, Id[] key, Domain[] domains,
            Topology[] topologies, byte[][] originals, ConceptualSlot slot,
            SystemPackageTopology.Limits limits) { }

    private record Held(ProjectedPlacement output, Id[] ids, byte[][] idBytes, byte[] hash,
            Domain domain, ConceptualSlot slot, Object[] outputBefore, Object[] gettersBefore) { }

    private record Context(Fixture fixture, int row, SystemPlacementRegistry registry,
            SystemPublicationHistory history, List<PublicationRecord> records,
            SystemPresentation client, List<Held> outputs) { }

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
        Object[][] topologies = new Object[2][];
        byte[][] originals = new byte[2][];
        for (int i = 0; i < 2; i++) {
            topologies[i] = topologyImage(f.topologies()[i]);
            originals[i] = f.originals()[i].clone();
        }
        return new Object[] {idImage(f.universe()), keyImage(f.key()),
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

    private static void fixedInputs(Fixture f, int row) {
        check(f.universe().equals(id(0x11)) && f.slot().value() == SLOTS[row], "fixed namespace and unsigned slot");
        check(f.key().length == 3, "complete fixed query key");
        for (int i = 0; i < 3; i++) {
            check(f.key()[i].equals(id(new int[] {0x22, 0x33, 0x44}[i])), "fixed query IDs");
        }
        for (int i = 0; i < 2; i++) {
            Topology t = f.topologies()[i];
            check(Arrays.equals(f.originals()[i], bytes(HEX[i])) && f.originals()[i].length == 199,
                    "independent complete original body");
            check(t.universe().equals(id(0x11)) && t.history().equals(id(0x22))
                    && t.owner().equals(id(0x33)) && t.system().equals(id(0x44)),
                    "fixed topology full identity");
            check(t.revision() == i + 1 && t.campaignDayMillis() == 2, "fixed revision and day");
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

    private static void fixedRegistry(SystemPlacementRegistry registry, int row, boolean populated) {
        check(registry.universe().equals(id(0x11)) && registry.capacity() == 1,
                "fixed registry namespace and capacity");
        check(registry.entries().size() == (populated ? 1 : 0), "independent exact registry row count");
        if (populated) {
            PlacementEntry e = registry.entries().get(0);
            check(e.history().equals(id(0x22)) && e.owner().equals(id(0x33))
                    && e.system().equals(id(0x44)), "full ordered registry key");
            check(e.slot().value() == SLOTS[row] && e.revision() == 1
                    && Arrays.equals(e.topologySha256(), bytes(SHA[0])), "fixed row slot revision and hash");
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

    private static void fixedOutput(ProjectedPlacement output, int row) {
        check(output.profile() == Profile.GRID_256_V1 && output.universe().equals(id(0x11)),
                "fixed output profile and universe");
        check(output.history().equals(id(0x22)) && output.owner().equals(id(0x33))
                && output.system().equals(id(0x44)), "fixed output full key");
        check(output.slot().value() == SLOTS[row] && output.revision() == 1
                && Arrays.equals(output.topologySha256(), bytes(SHA[0])),
                "fixed owned output unsigned slot revision and hash");
        check(Arrays.equals(domainImage(output.domain()), expectedDomain(CLIENTS[row]))
                && output.x() == X[row] && output.y() == Y[row],
                "independent complete epoch and fixed coordinates");
    }

    private static void fixedOutputs(Context context) {
        for (Held h : context.outputs()) {
            fixedOutput(h.output(), context.row());
            check(Arrays.deepEquals(h.outputBefore(), outputImage(h.output()))
                    && Arrays.deepEquals(h.gettersBefore(),
                            getterImage(h.ids(), h.idBytes(), h.hash(), h.domain(), h.slot())),
                    "held output and every retained getter value remain unchanged");
            int[] ids = {0x11, 0x22, 0x33, 0x44};
            for (int j = 0; j < 4; j++) {
                check(h.ids()[j].equals(id(ids[j]))
                        && Arrays.equals(h.idBytes()[j], id(ids[j]).value()), "fixed held IDs and arrays");
            }
            check(Arrays.equals(h.hash(), bytes(SHA[0]))
                    && Arrays.equals(domainImage(h.domain()), expectedDomain(CLIENTS[context.row()]))
                    && h.slot().value() == SLOTS[context.row()], "fixed retained hash domain and slot");
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
                recordImages(c.records()), layoutImage(c.client())};
    }

    private static Object[] sourceImage(Context c) {
        return new Object[] {registryImage(c.registry()), immutableSources(c)};
    }

    private static void fixedContext(Context c, boolean populated) {
        fixedInputs(c.fixture(), c.row());
        fixedRegistry(c.registry(), c.row(), populated);
        fixedHistory(c.history(), 2);
        check(c.records().size() == 2, "retained History list length");
        fixedRecord(c.records().get(0), 1);
        fixedRecord(c.records().get(1), 2);
        check(c.client().profile() == Profile.GRID_256_V1 && c.client().universe().equals(id(0x11))
                && Arrays.equals(domainImage(c.client().domain()), expectedDomain(CLIENTS[c.row()])),
                "complete fixed selected epoch identity");
        fixedOutputs(c);
    }

    private static Object[][] previousImages(List<Context> previous) {
        Object[][] images = new Object[previous.size()][];
        for (int i = 0; i < images.length; i++) {
            Context c = previous.get(i);
            images[i] = new Object[] {sourceImage(c), heldImages(c.outputs())};
        }
        return images;
    }

    private static void previousUnchanged(List<Context> previous, Object[][] before) {
        check(Arrays.deepEquals(before, previousImages(previous)), "all earlier fixtures and output copies unchanged");
        for (Context c : previous) {
            fixedContext(c, true);
            check(c.outputs().size() == 1, "one retained actual result per earlier fixture");
        }
    }

    private static Context fresh(int row) {
        byte[][] originals = {bytes(HEX[0]), bytes(HEX[1])};
        Topology[] topologies = new Topology[2];
        for (int i = 0; i < 2; i++) {
            topologies[i] = SystemPackageTopology.decode(originals[i], SystemPackageTopology.DEFAULT);
        }
        Fixture f = new Fixture(id(0x11), new Id[] {id(0x22), id(0x33), id(0x44)},
                new Domain[] {new Domain(-1024, -2048, 8192, 16384, 7),
                    new Domain(4096, -4096, 16384, 8192, 7)},
                topologies, originals, new ConceptualSlot(SLOTS[row]), SystemPackageTopology.DEFAULT);
        fixedInputs(f, row);
        Object[] inputsBefore = inputImage(f);
        // Independent History is retained only as a guard, never as projection admission.
        SystemPublicationHistory history = new SystemPublicationHistory(f.universe(),
                new SystemPublicationHistory.Limits(2, 3, 597));
        check(history.appendPublication(topologies[0], f.limits())
                == SystemPublicationHistory.Outcome.PUBLICATION_APPENDED, "actual held A1 seed");
        fixedHistory(history, 1);
        check(history.appendPublication(topologies[1], f.limits())
                == SystemPublicationHistory.Outcome.PUBLICATION_APPENDED, "actual held A2 seed");
        fixedHistory(history, 2);
        Context context = new Context(f, row, new SystemPlacementRegistry(f.universe(), 1),
                history, history.records(), new SystemPresentation(f.universe(), f.domains()[CLIENTS[row]]),
                new ArrayList<>());
        check(Arrays.deepEquals(inputsBefore, inputImage(f)), "all setup inputs unchanged");
        fixedContext(context, false);
        return context;
    }

    private static void projectUnsignedSlotExtrema() {
        List<Context> previous = new ArrayList<>();
        int completed = 0;
        for (int row = 0; row < 4; row++) {
            Object[][] previousBefore = previousImages(previous);
            Context context = fresh(row);
            previousUnchanged(previous, previousBefore);
            Fixture f = context.fixture();
            Object[] immutableBeforeReserve = immutableSources(context);
            check(context.registry().reserve(f.topologies()[0], f.slot(), f.limits())
                    == SystemPlacementRegistry.ReservationOutcome.INSERTED, "actual fresh capacity1 A1 reserve");
            fixedContext(context, true);
            check(Arrays.deepEquals(immutableBeforeReserve, immutableSources(context)),
                    "only selected registry changes during reserve");
            previousUnchanged(previous, previousBefore);

            Object[] beforeProject = sourceImage(context);
            Id[] key = f.key();
            ProjectedPlacement output = context.client().project(context.registry(), key[0], key[1], key[2]);
            fixedOutput(output, row);
            context.outputs().add(hold(output));
            defensiveCopies(output);
            fixedContext(context, true);
            check(context.outputs().size() == 1, "one actual positive project in fresh registry");
            check(Arrays.deepEquals(beforeProject, sourceImage(context)),
                    "full fixed registry, epoch, topology, query, limits and History unchanged after project");
            previousUnchanged(previous, previousBefore);
            previous.add(context);
            completed++;
        }
        check(completed == 4 && previous.size() == 4, "four fresh registries, four actual positive projects");
        previousUnchanged(previous, previousImages(previous));
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        projectUnsignedSlotExtrema();
        System.out.println("Grid256V1 Java unsigned slot extrema preserve client outputs PASS; data only");
    }
}
