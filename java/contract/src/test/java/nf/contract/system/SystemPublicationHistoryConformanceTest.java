package nf.contract.system;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.List;
import nf.contract.system.SystemPackageTopology.Dependency;
import nf.contract.system.SystemPackageTopology.Entity;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPackageTopology.Key;
import nf.contract.system.SystemPackageTopology.Topology;
import nf.contract.system.SystemPublicationHistory.Failure;
import nf.contract.system.SystemPublicationHistory.Kind;
import nf.contract.system.SystemPublicationHistory.Limits;
import nf.contract.system.SystemPublicationHistory.Outcome;
import nf.contract.system.SystemPublicationHistory.Profile;
import nf.contract.system.SystemPublicationHistory.PublicationError;
import nf.contract.system.SystemPublicationHistory.PublicationRecord;

/** Four bounded history conformance groups; selected public data only, no authority. */
public final class SystemPublicationHistoryConformanceTest {
    private SystemPublicationHistoryConformanceTest() { }

    private record Fixture(String name, String hex, String hash, int history, int owner,
            int system, long revision, long day, byte[] original, Topology input) { }
    private record Row(Fixture publication, Kind kind, long revision) { }
    private record State(int keys, int bytes, String rows, String latest) { }
    private record Input(Topology topology, SystemPackageTopology.Limits topologyLimits,
            Id history, Id owner, Id system, long revision, boolean publication) { }
    private record Held(List<PublicationRecord> rows, Object[][] images, Object[][] outputs,
            List<List<Id>> identifiers) { }

    private static final Fixture A1 = fixture("A1",
            "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183", 0x22, 0x33, 0x44, 1, 2);
    private static final Fixture A2 = fixture("A2",
            "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d", 0x22, 0x33, 0x44, 2, 2);
    private static final Fixture C2 = fixture("C2",
            "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000030000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "52ed6079f4b31e1aa71f17a3740bfd085f56bc6bd8f4df1bb4ec31ae136fa5be", 0x22, 0x33, 0x44, 2, 3);
    private static final Fixture AH = fixture("AH",
            "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440000000000000080ffffffffffffffff01010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "840c1706efbc0e083b7743c01e9964f65b7698767e5bb5f983e7a2762b77fc53", 0x22, 0x33, 0x44, Long.MIN_VALUE, -1);
    private static final Fixture AM = fixture("AM",
            "4e462d5350542d3100010011111111111111111111111111111111222222222222222222222222222222223333333333333333333333333333333344444444444444444444444444444444ffffffffffffffffffffffffffffffff01010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "af7535b29a044a03adf92107775dd55b2c4e14f03fac34ecca4e216d53151345", 0x22, 0x33, 0x44, -1, -1);
    private static final Fixture B1 = fixture("B1",
            "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333555555555555555555555555555555550100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "6ea8fd3113fc299431367483e01f808086cdc28062ac9daf9d03a761fdcde821", 0x22, 0x33, 0x55, 1, 2);
    private static final Fixture A4 = fixture("A4",
            "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440400000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "1fa957a0c4e5780f76d86618a6617482c9ce7fff1bad4d534c85085423dc11e1", 0x22, 0x33, 0x44, 4, 2);
    private static final Fixture A5 = fixture("A5",
            "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440500000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "704bed23c8ed3a9d2b3fc3fc0393834a9f972c0d8c6203fc4b770ab7deb9b732", 0x22, 0x33, 0x44, 5, 2);
    private static final Fixture H7 = fixture("H7",
            "4e462d5350542d31000100111111111111111111111111111111117f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7ffefefefefefefefefefefefefefefefefefefefefefefefefefefefefefefefe0100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "8100c2283ecae91958b27c51ecfeb2929f6043abf3eed3e647a15a9d95ba605c", 0x7f, 0xfe, 0xfe, 1, 2);
    private static final Fixture H8 = fixture("H8",
            "4e462d5350542d31000100111111111111111111111111111111118080808080808080808080808080808001010101010101010101010101010101010101010101010101010101010101010100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "d7186064df291f76d01812d127343cd4018e2f878db7246699063d8fea90dd46", 0x80, 0x1, 0x1, 1, 2);
    private static final Fixture O7 = fixture("O7",
            "4e462d5350542d3100010011111111111111111111111111111111222222222222222222222222222222227f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7ffefefefefefefefefefefefefefefefe0100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "a340e718ecda294ff8d6777e9b29cdc514a1f55eb1d81e73c38d9a684f2d2aee", 0x22, 0x7f, 0xfe, 1, 2);
    private static final Fixture O8 = fixture("O8",
            "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222280808080808080808080808080808080010101010101010101010101010101010100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "dd259684a3b3516aad76496ab44c036d0a4e19c52db748d5a635abd3e3b493bb", 0x22, 0x80, 0x1, 1, 2);
    private static final Fixture S7 = fixture("S7",
            "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222222222333333333333333333333333333333337f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f0100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "bc100d592da430de5eedd1c87a228a1f0073d6d2e9912438deffd2bfbd1ecb24", 0x22, 0x33, 0x7f, 1, 2);
    private static final Fixture S8 = fixture("S8",
            "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333808080808080808080808080808080800100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64",
            "c2dacf84b51533e15caedd89d249aa45907d85771fd4e226bdb481fb6172e5d0", 0x22, 0x33, 0x80, 1, 2);
    private static final List<Fixture> ALL = List.of(A1, A2, C2, AH, AM, B1, A4, A5,
            H7, H8, O7, O8, S7, S8);
    private static final List<Fixture> KEYS = List.of(A1, B1, S7, S8, O7, O8, H7, H8);

    private static void check(boolean value, String reason) {
        if (!value) throw new AssertionError(reason);
    }

    private static byte[] bytes(String hex) { return HexFormat.of().parseHex(hex); }
    private static Id id(int value) {
        byte[] raw = new byte[16];
        Arrays.fill(raw, (byte) value);
        return new Id(raw);
    }
    private static byte[] value(Id id) { return id == null ? null : id.value().clone(); }
    private static Object[] limitImage(Limits limits) {
        return limits == null ? null : new Object[] {
            limits.keys(), limits.records(), limits.canonicalBytes()
        };
    }
    private static Object[] topologyLimitsImage(SystemPackageTopology.Limits limits) {
        return limits == null ? null : new Object[] {
            limits.entities(), limits.dependencies(), limits.labelBytes(), limits.documentBytes()
        };
    }
    private static Object[] topologyImage(Topology input) {
        if (input == null) return null;
        Object[][] dependencies = new Object[input.dependencies().size()][];
        for (int i = 0; i < dependencies.length; i++) {
            Dependency d = input.dependencies().get(i);
            dependencies[i] = new Object[] {d.semanticKey().value().clone(), d.required()};
        }
        Object[][] entities = new Object[input.entities().size()][];
        for (int i = 0; i < entities.length; i++) {
            Entity e = input.entities().get(i);
            entities[i] = new Object[] {value(e.id()), e.kind(), value(e.parent()),
                e.dependency(), e.label()};
        }
        return new Object[] {value(input.universe()), value(input.history()), value(input.owner()),
            value(input.system()), input.revision(), input.campaignDayMillis(), dependencies, entities};
    }
    private static Fixture fixture(String name, String hex, String hash, int history,
            int owner, int system, long revision, long day) {
        byte[] fixed = bytes(hex);
        Topology input = SystemPackageTopology.decode(fixed, SystemPackageTopology.DEFAULT);
        check(fixed.length == 199 && Arrays.equals(fixed, bytes(hex)), "fixed decoder input");
        check(Arrays.equals(SystemPackageTopology.encode(input, SystemPackageTopology.DEFAULT),
                bytes(hex)), "fixed canonical bytes");
        check(Arrays.equals(SystemPackageTopology.digest(input, SystemPackageTopology.DEFAULT),
                bytes(hash)), "independent fixed SHA");
        check(input.universe().equals(id(0x11)) && input.history().equals(id(history))
                && input.owner().equals(id(owner)) && input.system().equals(id(system)), "fixed identities");
        check(input.revision() == revision && input.campaignDayMillis() == day, "fixed unsigned fields");
        check(input.dependencies().size() == 1 && input.entities().size() == 2, "fixed graph counts");
        Dependency d = input.dependencies().get(0);
        byte[] key = new byte[32];
        Arrays.fill(key, (byte) 0x77);
        check(Arrays.equals(d.semanticKey().value(), key) && d.required(), "fixed dependency");
        Entity star = input.entities().get(0), planet = input.entities().get(1);
        check(star.id().equals(id(1)) && star.kind() == SystemPackageTopology.Kind.STAR
                && star.parent() == null && star.dependency() == null && "Sun".equals(star.label()),
                "fixed first entity");
        check(planet.id().equals(id(2)) && planet.kind() == SystemPackageTopology.Kind.PLANET
                && planet.parent().equals(id(1)) && Integer.valueOf(0).equals(planet.dependency())
                && "World".equals(planet.label()), "fixed second entity");
        return new Fixture(name, hex, hash, history, owner, system, revision, day, fixed, input);
    }
    private static Object[][] allInputs() {
        Object[][] images = new Object[ALL.size()][];
        for (int i = 0; i < images.length; i++) {
            Fixture f = ALL.get(i);
            check(Arrays.equals(f.original(), bytes(f.hex())), "original fixture array unchanged");
            images[i] = new Object[] {f.original().clone(), topologyImage(f.input())};
        }
        return images;
    }
    private static Object[] recordImage(PublicationRecord record) {
        if (record == null) return null;
        byte[] body = record.canonicalTopologyBytes();
        return new Object[] {value(record.universe()), value(record.history()), value(record.owner()),
            value(record.system()), record.profile(), record.kind(), record.revision(),
            record.referencedPublicationRevision(), record.referencedTopologySha256().clone(),
            body == null ? null : body.clone()};
    }
    private static Object[][] recordImages(List<PublicationRecord> records) {
        Object[][] images = new Object[records.size()][];
        for (int i = 0; i < images.length; i++) images[i] = recordImage(records.get(i));
        return images;
    }
    private static Object[] historyImage(SystemPublicationHistory history) {
        Object[][] latest = new Object[KEYS.size()][];
        for (int i = 0; i < latest.length; i++) {
            Fixture key = KEYS.get(i);
            latest[i] = recordImage(history.latest(id(key.history()), id(key.owner()), id(key.system())));
        }
        return new Object[] {value(history.universe()), limitImage(history.limits()), history.keysLen(),
            history.canonicalBytesLen(), recordImages(history.records()), latest};
    }
    private static Object[] inputImage(Input input) {
        return new Object[] {topologyImage(input.topology()), topologyLimitsImage(input.topologyLimits()),
            value(input.history()), value(input.owner()), value(input.system()), input.revision(),
            input.publication(), allInputs()};
    }
    private static Fixture named(String name) {
        for (Fixture f : ALL) if (f.name().equals(name)) return f;
        throw new AssertionError("unknown fixed fixture");
    }
    private static Row row(String name) {
        return switch (name) {
            case "T2" -> new Row(A1, Kind.TOMBSTONE, 2);
            case "T3" -> new Row(A2, Kind.TOMBSTONE, 3);
            case "T4" -> new Row(A2, Kind.TOMBSTONE, 4);
            case "TM" -> new Row(A2, Kind.TOMBSTONE, -1);
            default -> new Row(named(name), Kind.PUBLISHED_VERSION, named(name).revision());
        };
    }
    private static List<String> names(String text) {
        return text.isEmpty() ? List.of() : List.of(text.split(" "));
    }
    private static boolean sameKey(Fixture a, Fixture b) {
        return a.history() == b.history() && a.owner() == b.owner() && a.system() == b.system();
    }
    private static void record(PublicationRecord actual, Row expected) {
        check(actual != null, "expected actual record");
        Fixture f = expected.publication();
        check(actual.universe().equals(id(0x11)) && actual.history().equals(id(f.history()))
                && actual.owner().equals(id(f.owner())) && actual.system().equals(id(f.system())),
                "all four record IDs");
        check(actual.profile() == Profile.SELECTED_TOPOLOGY_V1 && actual.kind() == expected.kind(),
                "profile and kind");
        check(actual.revision() == expected.revision()
                && actual.referencedPublicationRevision() == f.revision(), "event and reference revisions");
        check(Arrays.equals(actual.referencedTopologySha256(), bytes(f.hash())), "fixed reference hash");
        check(Arrays.equals(actual.canonicalTopologyBytes(), expected.kind() == Kind.TOMBSTONE
                ? null : bytes(f.hex())), "exact fixed body or tombstone null");
    }
    private static void state(SystemPublicationHistory history, Limits limits, State expected) {
        check(Arrays.deepEquals(limitImage(history.limits()), limitImage(limits)), "all limits");
        check(history.universe().equals(id(0x11)) && history.keysLen() == expected.keys()
                && history.canonicalBytesLen() == expected.bytes(), "namespace and exact counters");
        List<PublicationRecord> records = history.records();
        List<String> order = names(expected.rows());
        check(records.size() == order.size(), "exact record count");
        for (int i = 0; i < order.size(); i++) record(records.get(i), row(order.get(i)));
        for (Fixture key : KEYS) {
            Row latest = null;
            for (String name : names(expected.latest())) {
                Row candidate = row(name);
                if (sameKey(key, candidate.publication())) {
                    check(latest == null, "independent latest table has one row per key");
                    latest = candidate;
                }
            }
            PublicationRecord actual = history.latest(id(key.history()), id(key.owner()), id(key.system()));
            if (latest == null) check(actual == null, "independently missing full key");
            else record(actual, latest);
        }
    }
    private static State expected(int keys, int bytes, String rows, String latest) {
        return new State(keys, bytes, rows, latest);
    }
    private static Input publication(Topology input, SystemPackageTopology.Limits limits) {
        return new Input(input, limits, null, null, null, 0, true);
    }
    private static Input publication(Fixture f) { return publication(f.input(), SystemPackageTopology.DEFAULT); }
    private static Input tombstone(Id h, Id o, Id s, long revision) {
        return new Input(null, null, h, o, s, revision, false);
    }
    private static Input tombstone(long revision) { return tombstone(id(0x22), id(0x33), id(0x44), revision); }

    private static final class Scenario {
        private final Limits limits;
        private final SystemPublicationHistory history;
        private final List<Held> held = new ArrayList<>();
        private State expected;
        private Scenario(int keys, int records, int bytes) {
            limits = new Limits(keys, records, bytes);
            Id universe = id(0x11);
            Object[] before = new Object[] {value(universe), limitImage(limits), allInputs()};
            history = new SystemPublicationHistory(universe, limits);
            check(Arrays.deepEquals(before, new Object[] {value(universe), limitImage(limits), allInputs()}),
                    "constructor inputs unchanged");
            expected = expected(0, 0, "", "");
            state(history, limits, expected);
            remember();
        }
        private void retained() {
            for (Held old : held) {
                check(Arrays.deepEquals(old.images(), recordImages(old.rows())), "held list and record values");
                for (int i = 0; i < old.outputs().length; i++) {
                    check(Arrays.deepEquals(old.outputs()[i], old.images()[i]), "held IDs/hash/body arrays");
                    for (int j = 0; j < 4; j++) {
                        check(Arrays.equals(old.identifiers().get(i).get(j).value(),
                                (byte[]) old.images()[i][j]), "held Id object retains its bytes");
                    }
                }
            }
        }
        private void remember() {
            List<PublicationRecord> records = history.records();
            Object[][] outputs = new Object[records.size()][];
            List<List<Id>> identifiers = new ArrayList<>();
            for (int i = 0; i < records.size(); i++) {
                PublicationRecord r = records.get(i);
                identifiers.add(List.of(r.universe(), r.history(), r.owner(), r.system()));
                outputs[i] = new Object[] {r.universe().value(), r.history().value(), r.owner().value(),
                    r.system().value(), r.profile(), r.kind(), r.revision(),
                    r.referencedPublicationRevision(), r.referencedTopologySha256(), r.canonicalTopologyBytes()};
            }
            boolean immutable = false;
            try { records.clear(); } catch (UnsupportedOperationException error) { immutable = true; }
            check(immutable, "records list unmodifiable");
            held.add(new Held(records, recordImages(records), outputs, List.copyOf(identifiers)));
        }
    }
    private static void step(Scenario scenario, Input input, Outcome outcome,
            PublicationError error, SystemPackageTopology.Code topologyCode, State after) {
        Object[] before = historyImage(scenario.history), inputs = inputImage(input);
        try {
            Outcome actual = input.publication()
                    ? scenario.history.appendPublication(input.topology(), input.topologyLimits())
                    : scenario.history.appendTombstone(input.history(), input.owner(), input.system(), input.revision());
            check(error == null && actual == outcome, "exact append outcome");
        } catch (Failure failure) {
            check(error != null && failure.code() == error && failure.topologyCode() == topologyCode,
                    "exact failure and topology code");
            check(failure.getMessage().equals(error.name()) && failure.getCause() == null,
                    "exact fixed message and null cause");
        }
        check(Arrays.deepEquals(inputs, inputImage(input)), "all actual argument and fixture images unchanged");
        if (error != null || outcome == Outcome.UNCHANGED) {
            check(Arrays.deepEquals(before, historyImage(scenario.history)), "whole receiver nonmutation");
        }
        state(scenario.history, scenario.limits, after);
        scenario.expected = after;
        scenario.retained();
        scenario.remember();
    }
    private static void publish(Scenario s, Fixture input, State after) {
        step(s, publication(input), Outcome.PUBLICATION_APPENDED, null, null, after);
    }
    private static void tombstone(Scenario s, long revision, State after) {
        step(s, tombstone(revision), Outcome.TOMBSTONE_APPENDED, null, null, after);
    }
    private static void unchanged(Scenario s, Input input) {
        step(s, input, Outcome.UNCHANGED, null, null, s.expected);
    }
    private static void refuse(Scenario s, Input input, PublicationError code) {
        step(s, input, null, code, null, s.expected);
    }
    private static void topologyRefusal(Scenario s, Topology input,
            SystemPackageTopology.Limits limits, SystemPackageTopology.Code code) {
        step(s, publication(input, limits), null, PublicationError.TOPOLOGY, code, s.expected);
    }
    private static void constructorRefusal(Id universe, Limits limits, PublicationError code) {
        Object[] before = new Object[] {value(universe), limitImage(limits), allInputs()};
        boolean failed = false;
        try { new SystemPublicationHistory(universe, limits); }
        catch (Failure failure) {
            failed = true;
            check(failure.code() == code && failure.topologyCode() == null
                    && failure.getMessage().equals(code.name()) && failure.getCause() == null,
                    "exact constructor failure");
        }
        check(failed, "constructor must refuse");
        check(Arrays.deepEquals(before, new Object[] {value(universe), limitImage(limits), allInputs()}),
                "constructor arguments unchanged without fictional receiver");
    }
    private static Topology direct(Id universe, long revision) {
        Topology a = A1.input();
        return new Topology(universe, a.history(), a.owner(), a.system(), revision,
                a.campaignDayMillis(), a.dependencies(), a.entities());
    }
    private static void query(Scenario s, Id h, Id o, Id system, Row expected) {
        Object[] before = historyImage(s.history);
        Object[] arguments = new Object[] {value(h), value(o), value(system), allInputs()};
        PublicationRecord actual = s.history.latest(h, o, system);
        if (expected == null) check(actual == null, "missing query returns null");
        else record(actual, expected);
        check(Arrays.deepEquals(before, historyImage(s.history)), "query receiver unchanged");
        check(Arrays.deepEquals(arguments, new Object[] {value(h), value(o), value(system), allInputs()}),
                "query arguments unchanged");
        state(s.history, s.limits, s.expected);
        s.retained();
    }
    private static void ownership(Scenario s) {
        Object[] before = historyImage(s.history);
        Object[][] inputs = allInputs();
        Arrays.fill(s.history.universe().value(), (byte) 0);
        for (PublicationRecord r : s.history.records()) {
            for (Id id : List.of(r.universe(), r.history(), r.owner(), r.system())) {
                Arrays.fill(id.value(), (byte) 0);
            }
            Arrays.fill(r.referencedTopologySha256(), (byte) 0);
            byte[] body = r.canonicalTopologyBytes();
            if (body != null) Arrays.fill(body, (byte) 0);
        }
        for (Fixture f : ALL) {
            Topology t = f.input();
            for (Id id : List.of(t.universe(), t.history(), t.owner(), t.system())) {
                Arrays.fill(id.value(), (byte) 0);
            }
            for (Dependency d : t.dependencies()) Arrays.fill(d.semanticKey().value(), (byte) 0);
            for (Entity e : t.entities()) {
                Arrays.fill(e.id().value(), (byte) 0);
                if (e.parent() != null) Arrays.fill(e.parent().value(), (byte) 0);
            }
        }
        check(Arrays.deepEquals(before, historyImage(s.history)), "output mutation cannot alter receiver");
        check(Arrays.deepEquals(inputs, allInputs()), "nested input getters are defensive");
        state(s.history, s.limits, s.expected);
        s.retained();
    }

    private static void admission() {
        constructorRefusal(id(0), new Limits(0, 0, 0), PublicationError.INVALID_NAMESPACE);
        constructorRefusal(null, new Limits(0, 0, 0), PublicationError.INVALID_NAMESPACE);
        for (Limits bad : List.of(new Limits(0, 1, 199), new Limits(257, 1, 199),
                new Limits(1, 0, 199), new Limits(1, 513, 199), new Limits(1, 1, 0),
                new Limits(1, 1, 8_388_609))) {
            constructorRefusal(id(0x11), bad, PublicationError.BOUNDS);
        }
        constructorRefusal(id(0x11), null, PublicationError.BOUNDS);
        Scenario maximum = new Scenario(256, 512, 8_388_608);
        query(maximum, id(0x22), id(0x33), id(0x44), null);
        Scenario independent = new Scenario(256, 1, 1);
        query(independent, id(0x22), id(0x33), id(0x44), null);
        Scenario empty = new Scenario(1, 1, 1);
        refuse(empty, tombstone(id(0), id(0x33), id(0x44), 1), PublicationError.INVALID_INPUT);
        refuse(empty, tombstone(id(0x22), id(0), id(0x44), 1), PublicationError.INVALID_INPUT);
        refuse(empty, tombstone(id(0x22), id(0x33), id(0), 1), PublicationError.INVALID_INPUT);
        refuse(empty, tombstone(0), PublicationError.INVALID_INPUT);
        refuse(empty, tombstone(null, id(0x33), id(0x44), 1), PublicationError.INVALID_INPUT);
        refuse(empty, tombstone(id(0x22), null, id(0x44), 1), PublicationError.INVALID_INPUT);
        refuse(empty, tombstone(id(0x22), id(0x33), null, 1), PublicationError.INVALID_INPUT);
        refuse(empty, tombstone(1), PublicationError.MISSING_PUBLICATION);
        Scenario full = new Scenario(1, 1, 199);
        publish(full, A1, expected(1, 199, "A1", "A1"));
        // Direct invalid objects are imaged only; no pre-call validation/encoding/decoding.
        topologyRefusal(full, direct(id(0x99), 0), SystemPackageTopology.DEFAULT,
                SystemPackageTopology.Code.INVALID);
        topologyRefusal(full, A1.input(), new SystemPackageTopology.Limits(128, 64, 128, 198),
                SystemPackageTopology.Code.LIMIT);
        topologyRefusal(full, A1.input(), new SystemPackageTopology.Limits(0, 64, 128, 65536),
                SystemPackageTopology.Code.LIMIT);
        refuse(full, publication(direct(id(0x99), 1), SystemPackageTopology.DEFAULT),
                PublicationError.UNIVERSE_MISMATCH);
        topologyRefusal(full, null, SystemPackageTopology.DEFAULT, SystemPackageTopology.Code.INVALID);
        topologyRefusal(full, A1.input(), null, SystemPackageTopology.Code.INVALID);
        byte[] semantic = new byte[32];
        Arrays.fill(semantic, (byte) 0x77);
        Topology cycle = new Topology(id(0x11), id(0x22), id(0x33), id(0x44), 1, 2,
                List.of(new Dependency(new Key(semantic), true)), List.of(
                    new Entity(id(1), SystemPackageTopology.Kind.STAR, null, null, "Sun"),
                    new Entity(id(2), SystemPackageTopology.Kind.PLANET, id(3), 0, "World"),
                    new Entity(id(3), SystemPackageTopology.Kind.STATION, id(2), null, null)));
        topologyRefusal(full, cycle, SystemPackageTopology.DEFAULT, SystemPackageTopology.Code.CYCLE);
        ownership(full);
    }

    private static void quotas() {
        Scenario key = new Scenario(1, 2, 398);
        publish(key, A1, expected(1, 199, "A1", "A1"));
        publish(key, A2, expected(1, 398, "A1 A2", "A2"));
        refuse(key, publication(B1), PublicationError.KEY_LIMIT);

        Scenario records = new Scenario(2, 1, 199);
        publish(records, A1, expected(1, 199, "A1", "A1"));
        refuse(records, publication(A2), PublicationError.RECORD_LIMIT);

        Scenario bytes = new Scenario(1, 3, 199);
        publish(bytes, A1, expected(1, 199, "A1", "A1"));
        refuse(bytes, publication(A2), PublicationError.BYTE_LIMIT);

        Scenario zeroBytes = new Scenario(1, 3, 199);
        publish(zeroBytes, A1, expected(1, 199, "A1", "A1"));
        tombstone(zeroBytes, 2, expected(1, 199, "A1 T2", "T2"));
        ownership(zeroBytes);

        Scenario full = new Scenario(1, 3, 398);
        publish(full, A1, expected(1, 199, "A1", "A1"));
        publish(full, A2, expected(1, 398, "A1 A2", "A2"));
        tombstone(full, 3, expected(1, 398, "A1 A2 T3", "T3"));
        unchanged(full, publication(A1));
        refuse(full, publication(C2), PublicationError.REVISION_CONFLICT);

        Scenario stale = new Scenario(1, 2, 199);
        publish(stale, A2, expected(1, 199, "A2", "A2"));
        tombstone(stale, 3, expected(1, 199, "A2 T3", "T3"));
        refuse(stale, publication(A1), PublicationError.STALE_REVISION);

        // Five separately isolated supplemental retained-state refusals.
        Scenario newKeyRecord = new Scenario(2, 1, 199);
        publish(newKeyRecord, A1, expected(1, 199, "A1", "A1"));
        refuse(newKeyRecord, publication(B1), PublicationError.RECORD_LIMIT);
        query(newKeyRecord, id(0x22), id(0x33), id(0x55), null);
        Scenario newKeyBytes = new Scenario(2, 3, 199);
        publish(newKeyBytes, A1, expected(1, 199, "A1", "A1"));
        refuse(newKeyBytes, publication(B1), PublicationError.BYTE_LIMIT);
        query(newKeyBytes, id(0x22), id(0x33), id(0x55), null);
        Scenario tombRecord = new Scenario(1, 1, 199);
        publish(tombRecord, A1, expected(1, 199, "A1", "A1"));
        refuse(tombRecord, tombstone(2), PublicationError.RECORD_LIMIT);
        Scenario tombStale = new Scenario(1, 2, 199);
        publish(tombStale, A2, expected(1, 199, "A2", "A2"));
        tombstone(tombStale, 3, expected(1, 199, "A2 T3", "T3"));
        refuse(tombStale, tombstone(1), PublicationError.STALE_REVISION);
        Scenario tombMissing = new Scenario(1, 1, 199);
        publish(tombMissing, A1, expected(1, 199, "A1", "A1"));
        refuse(tombMissing, tombstone(id(0x22), id(0x33), id(0x55), 2),
                PublicationError.MISSING_PUBLICATION);
    }

    private static void unsignedAndMax() {
        Scenario s = new Scenario(2, 12, 2388);
        publish(s, A1, expected(1, 199, "A1", "A1"));
        publish(s, A2, expected(1, 398, "A1 A2", "A2"));
        refuse(s, publication(C2), PublicationError.REVISION_CONFLICT);
        tombstone(s, 3, expected(1, 398, "A1 A2 T3", "T3"));
        unchanged(s, publication(A1));
        unchanged(s, publication(A2));
        unchanged(s, tombstone(3));
        publish(s, A4, expected(1, 597, "A1 A2 T3 A4", "A4"));
        unchanged(s, tombstone(3));
        publish(s, AH, expected(1, 796, "A1 A2 T3 A4 AH", "AH"));
        refuse(s, publication(A5), PublicationError.STALE_REVISION);
        publish(s, AM, expected(1, 995, "A1 A2 T3 A4 AH AM", "AM"));
        unchanged(s, publication(AM));
        refuse(s, tombstone(-1), PublicationError.REVISION_CONFLICT);
        refuse(s, tombstone(Long.MIN_VALUE + 1), PublicationError.STALE_REVISION);
        refuse(s, tombstone(0), PublicationError.INVALID_INPUT);
        ownership(s);

        Scenario maximumTombstone = new Scenario(1, 3, 199);
        publish(maximumTombstone, A2, expected(1, 199, "A2", "A2"));
        tombstone(maximumTombstone, -1, expected(1, 199, "A2 TM", "TM"));
        unchanged(maximumTombstone, tombstone(-1));
        refuse(maximumTombstone, publication(AM), PublicationError.REVISION_CONFLICT);
        refuse(maximumTombstone, publication(A1), PublicationError.STALE_REVISION);
        unchanged(maximumTombstone, publication(A2));
        ownership(maximumTombstone);
    }

    private static void fullKeysAndTombstones() {
        Scenario reverseAB = new Scenario(2, 2, 398);
        publish(reverseAB, B1, expected(1, 199, "B1", "B1"));
        publish(reverseAB, A1, expected(2, 398, "A1 B1", "A1 B1"));
        Scenario forwardAB = new Scenario(2, 2, 398);
        publish(forwardAB, A1, expected(1, 199, "A1", "A1"));
        publish(forwardAB, B1, expected(2, 398, "A1 B1", "A1 B1"));
        query(forwardAB, id(0x22), id(0x33), id(0x44), row("A1"));
        query(forwardAB, id(0x22), id(0x33), id(0x55), row("B1"));
        query(forwardAB, id(0x22), id(0x33), id(0x66), null);
        query(forwardAB, id(0), id(0x33), id(0x44), null);
        query(forwardAB, id(0x22), id(0), id(0x44), null);
        query(forwardAB, id(0x22), id(0x33), id(0), null);
        query(forwardAB, null, id(0x33), id(0x44), null);
        query(forwardAB, id(0x22), null, id(0x44), null);
        query(forwardAB, id(0x22), id(0x33), null, null);

        Scenario lastByte = new Scenario(1, 1, 199);
        publish(lastByte, A1, expected(1, 199, "A1", "A1"));
        query(lastByte, new Id(bytes("22222222222222222222222222222223")), id(0x33), id(0x44), null);
        query(lastByte, id(0x22), new Id(bytes("33333333333333333333333333333334")), id(0x44), null);
        query(lastByte, id(0x22), id(0x33), new Id(bytes("44444444444444444444444444444445")), null);

        Scenario repeated = new Scenario(1, 3, 199);
        publish(repeated, A2, expected(1, 199, "A2", "A2"));
        tombstone(repeated, 3, expected(1, 199, "A2 T3", "T3"));
        tombstone(repeated, 4, expected(1, 199, "A2 T3 T4", "T4"));
        ownership(repeated);

        // Fixed stage tables: no expected sort, revision comparison, or hash from the subject.
        Scenario forward = new Scenario(8, 8, 1592);
        publish(forward, A1, expected(1, 199, "A1", "A1"));
        publish(forward, B1, expected(2, 398, "A1 B1", "A1 B1"));
        publish(forward, S7, expected(3, 597, "A1 B1 S7", "A1 B1 S7"));
        publish(forward, S8, expected(4, 796, "A1 B1 S7 S8", "A1 B1 S7 S8"));
        publish(forward, O7, expected(5, 995, "A1 B1 S7 S8 O7", "A1 B1 S7 S8 O7"));
        publish(forward, O8, expected(6, 1194, "A1 B1 S7 S8 O7 O8", "A1 B1 S7 S8 O7 O8"));
        publish(forward, H7, expected(7, 1393, "A1 B1 S7 S8 O7 O8 H7", "A1 B1 S7 S8 O7 O8 H7"));
        publish(forward, H8, expected(8, 1592, "A1 B1 S7 S8 O7 O8 H7 H8", "A1 B1 S7 S8 O7 O8 H7 H8"));
        ownership(forward);
        Scenario reverse = new Scenario(8, 8, 1592);
        publish(reverse, H8, expected(1, 199, "H8", "H8"));
        publish(reverse, H7, expected(2, 398, "H7 H8", "H7 H8"));
        publish(reverse, O8, expected(3, 597, "O8 H7 H8", "O8 H7 H8"));
        publish(reverse, O7, expected(4, 796, "O7 O8 H7 H8", "O7 O8 H7 H8"));
        publish(reverse, S8, expected(5, 995, "S8 O7 O8 H7 H8", "S8 O7 O8 H7 H8"));
        publish(reverse, S7, expected(6, 1194, "S7 S8 O7 O8 H7 H8", "S7 S8 O7 O8 H7 H8"));
        publish(reverse, B1, expected(7, 1393, "B1 S7 S8 O7 O8 H7 H8", "B1 S7 S8 O7 O8 H7 H8"));
        publish(reverse, A1, expected(8, 1592, "A1 B1 S7 S8 O7 O8 H7 H8", "A1 B1 S7 S8 O7 O8 H7 H8"));
        ownership(reverse);
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        admission();
        System.out.println("NF-SPT-1 Java history admission refusals PASS; data only");
        quotas();
        System.out.println("NF-SPT-1 Java history quota refusal atomicity PASS; data only");
        unsignedAndMax();
        System.out.println("NF-SPT-1 Java history unsigned MAX event identity PASS; data only");
        fullKeysAndTombstones();
        System.out.println("NF-SPT-1 Java history full key order and tombstone references PASS; data only");
    }
}
