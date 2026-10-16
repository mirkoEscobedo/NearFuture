package nf.contract.system;

import java.util.Arrays;
import java.util.List;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPackageTopology.Topology;
import nf.contract.system.SystemPublicationHistory.Kind;
import nf.contract.system.SystemPublicationHistory.Limits;
import nf.contract.system.SystemPublicationHistory.Outcome;
import nf.contract.system.SystemPublicationHistory.Profile;
import nf.contract.system.SystemPublicationHistory.PublicationRecord;

/** One immutable canonical-history parity scenario; no publication or game authority. */
public final class SystemPublicationHistoryTest {
    private SystemPublicationHistoryTest() { }

    private static final String A1_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440100000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A2_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440200000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A1_SHA = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
    private static final String A2_SHA = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
    private static final Limits HISTORY_LIMITS = new Limits(1, 3, 398);

    private static final String A4_HEX = "4e462d5350542d31000100111111111111111111111111111111112222222222222222222222222222222233333333333333333333333333333333444444444444444444444444444444440400000000000000020000000000000001010077777777777777777777777777777777777777777777777777777777777777770102000101010101010101010101010101010101000001030053756e02020202020202020202020202020202020101010101010101010101010101010101010000010500576f726c64";
    private static final String A4_SHA = "1fa957a0c4e5780f76d86618a6617482c9ce7fff1bad4d534c85085423dc11e1";
    private static final Limits REPLAY_LIMITS = new Limits(1, 4, 597);

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

    private static void inputUnchanged(Topology input, Topology original, byte[] literal, String hash) {
        check(input.equals(original), "whole input value unchanged");
        check(literal.length == 199, "199-byte input");
        SystemPackageTopology.validate(input, SystemPackageTopology.DEFAULT);
        check(Arrays.equals(SystemPackageTopology.encode(input, SystemPackageTopology.DEFAULT), literal), "input bytes");
        check(Arrays.equals(SystemPackageTopology.digest(input, SystemPackageTopology.DEFAULT), bytes(hash)), "input SHA");
    }

    private static void assertRecord(PublicationRecord record, Kind kind, long revision,
            long referencedRevision, String hash, byte[] body) {
        check(record.universe().equals(id(0x11)), "record universe");
        check(record.history().equals(id(0x22)), "record history");
        check(record.owner().equals(id(0x33)), "record owner");
        check(record.system().equals(id(0x44)), "record system");
        check(record.profile() == Profile.SELECTED_TOPOLOGY_V1, "selected profile");
        check(record.kind() == kind, "record kind");
        check(record.revision() == revision, "event revision");
        check(record.referencedPublicationRevision() == referencedRevision, "referenced publication revision");
        check(Arrays.equals(record.referencedTopologySha256(), bytes(hash)), "referenced fixed SHA");
        check(Arrays.equals(record.canonicalTopologyBytes(), body), "owned canonical body or null");
    }

    private static void counts(SystemPublicationHistory history, int records, int canonicalBytes) {
        check(history.universe().equals(id(0x11)), "history namespace");
        check(history.limits().equals(HISTORY_LIMITS), "explicit limits");
        check(history.keysLen() == 1, "one key");
        check(history.records().size() == records, "record count");
        check(history.canonicalBytesLen() == canonicalBytes, "canonical byte count");
    }

    private static void latest(SystemPublicationHistory history, int index) {
        check(history.latest(id(0x22), id(0x33), id(0x44)).equals(history.records().get(index)), "latest record");
    }

    private static void unmodifiable(List<PublicationRecord> records) {
        boolean refused = false;
        try {
            records.clear();
        } catch (UnsupportedOperationException expected) {
            refused = true;
        }
        check(refused, "records snapshot is unmodifiable");
    }

    private static void immutableVersionsAndTombstonePreserveCanonicalHistory() {
        byte[] a1Bytes = bytes(A1_HEX), a2Bytes = bytes(A2_HEX);
        Topology a1 = SystemPackageTopology.decode(a1Bytes, SystemPackageTopology.DEFAULT);
        Topology a2 = SystemPackageTopology.decode(a2Bytes, SystemPackageTopology.DEFAULT);
        Topology originalA1 = SystemPackageTopology.decode(bytes(A1_HEX), SystemPackageTopology.DEFAULT);
        Topology originalA2 = SystemPackageTopology.decode(bytes(A2_HEX), SystemPackageTopology.DEFAULT);
        inputUnchanged(a1, originalA1, a1Bytes, A1_SHA);
        inputUnchanged(a2, originalA2, a2Bytes, A2_SHA);
        check(a1.revision() == 1 && a2.revision() == 2, "fixed input revisions");
        check(a1.campaignDayMillis() == 2 && a2.campaignDayMillis() == 2, "descriptive input date");

        SystemPublicationHistory history = new SystemPublicationHistory(id(0x11), HISTORY_LIMITS);
        check(history.appendPublication(a1, SystemPackageTopology.DEFAULT) == Outcome.PUBLICATION_APPENDED, "append P1");
        counts(history, 1, 199);
        assertRecord(history.records().get(0), Kind.PUBLISHED_VERSION, 1, 1, A1_SHA, a1Bytes);
        latest(history, 0);
        List<PublicationRecord> firstRows = history.records();
        PublicationRecord firstRecord = firstRows.get(0);
        byte[] firstBody = firstRecord.canonicalTopologyBytes();
        byte[] firstHash = firstRecord.referencedTopologySha256();
        unmodifiable(firstRows);

        check(history.appendPublication(a2, SystemPackageTopology.DEFAULT) == Outcome.PUBLICATION_APPENDED, "append P2");
        counts(history, 2, 398);
        assertRecord(history.records().get(0), Kind.PUBLISHED_VERSION, 1, 1, A1_SHA, a1Bytes);
        assertRecord(history.records().get(1), Kind.PUBLISHED_VERSION, 2, 2, A2_SHA, a2Bytes);
        latest(history, 1);
        check(firstRows.size() == 1, "first list is not a live view");
        assertRecord(firstRecord, Kind.PUBLISHED_VERSION, 1, 1, A1_SHA, a1Bytes);
        List<PublicationRecord> secondRows = history.records();
        unmodifiable(secondRows);
        check(history.canonicalBytesLen() == history.limits().canonicalBytes(), "byte quota full before T3");

        check(history.appendTombstone(id(0x22), id(0x33), id(0x44), 3) == Outcome.TOMBSTONE_APPENDED, "append T3 at full bytes");
        counts(history, 3, 398);
        assertRecord(history.records().get(0), Kind.PUBLISHED_VERSION, 1, 1, A1_SHA, a1Bytes);
        assertRecord(history.records().get(1), Kind.PUBLISHED_VERSION, 2, 2, A2_SHA, a2Bytes);
        assertRecord(history.records().get(2), Kind.TOMBSTONE, 3, 2, A2_SHA, null);
        latest(history, 2);
        check(firstRows.size() == 1 && secondRows.size() == 2, "old list snapshots retained");
        check(history.records().subList(0, 2).equals(secondRows), "whole old records retained");
        assertRecord(firstRows.get(0), Kind.PUBLISHED_VERSION, 1, 1, A1_SHA, a1Bytes);
        assertRecord(secondRows.get(1), Kind.PUBLISHED_VERSION, 2, 2, A2_SHA, a2Bytes);
        check(Arrays.equals(firstBody, a1Bytes), "old body snapshot retained");
        check(Arrays.equals(firstHash, bytes(A1_SHA)), "old hash snapshot retained");
        unmodifiable(history.records());

        byte[] returnedBody = history.records().get(0).canonicalTopologyBytes();
        byte[] returnedHash = history.records().get(0).referencedTopologySha256();
        Arrays.fill(returnedBody, (byte) 0);
        Arrays.fill(returnedHash, (byte) 0);
        for (Id returnedId : List.of(history.universe(), firstRecord.universe(), firstRecord.history(),
                firstRecord.owner(), firstRecord.system())) {
            byte[] returnedValue = returnedId.value();
            Arrays.fill(returnedValue, (byte) 0);
        }
        counts(history, 3, 398);
        assertRecord(history.records().get(0), Kind.PUBLISHED_VERSION, 1, 1, A1_SHA, a1Bytes);
        assertRecord(history.records().get(1), Kind.PUBLISHED_VERSION, 2, 2, A2_SHA, a2Bytes);
        assertRecord(history.records().get(2), Kind.TOMBSTONE, 3, 2, A2_SHA, null);
        assertRecord(firstRecord, Kind.PUBLISHED_VERSION, 1, 1, A1_SHA, a1Bytes);
        check(Arrays.equals(firstBody, a1Bytes), "old body remains owned after output mutation");
        check(Arrays.equals(firstHash, bytes(A1_SHA)), "old hash remains owned after output mutation");
        check(history.records().subList(0, 2).equals(secondRows), "snapshot unaffected by output mutation");
        latest(history, 2);
        inputUnchanged(a1, originalA1, a1Bytes, A1_SHA);
        inputUnchanged(a2, originalA2, a2Bytes, A2_SHA);
        check(Arrays.equals(a1Bytes, bytes(A1_HEX)), "original A1 bytes preserved");
        check(Arrays.equals(a2Bytes, bytes(A2_HEX)), "original A2 bytes preserved");
    }

    private static Object[] replayRecordImage(PublicationRecord record) {
        byte[] body = record.canonicalTopologyBytes();
        return new Object[] {
            record.universe().value().clone(), record.history().value().clone(),
            record.owner().value().clone(), record.system().value().clone(),
            record.profile(), record.kind(), record.revision(), record.referencedPublicationRevision(),
            record.referencedTopologySha256().clone(), body == null ? null : body.clone()
        };
    }

    private static Object[][] replayRecordImages(List<PublicationRecord> records) {
        Object[][] images = new Object[records.size()][];
        for (int i = 0; i < records.size(); i++) images[i] = replayRecordImage(records.get(i));
        return images;
    }

    private static Object[] replayHistoryImage(SystemPublicationHistory history) {
        List<PublicationRecord> records = history.records();
        return new Object[] {
            history.universe().value().clone(), history.limits(), history.keysLen(),
            history.canonicalBytesLen(), records.size(), replayRecordImages(records),
            replayRecordImage(history.latest(id(0x22), id(0x33), id(0x44)))
        };
    }

    private static Object[][] replayInputImages(Topology[] inputs, Topology[] originals, byte[][] fixed) {
        String[] hexes = {A1_HEX, A2_HEX, A4_HEX};
        String[] hashes = {A1_SHA, A2_SHA, A4_SHA};
        long[] revisions = {1, 2, 4};
        Object[][] images = new Object[3][];
        for (int i = 0; i < 3; i++) {
            check(Arrays.equals(fixed[i], bytes(hexes[i])), "replay fixture bytes remain fixed");
            check(originals[i].equals(SystemPackageTopology.decode(bytes(hexes[i]), SystemPackageTopology.DEFAULT)), "replay independent original");
            inputUnchanged(inputs[i], originals[i], bytes(hexes[i]), hashes[i]);
            check(inputs[i].revision() == revisions[i] && inputs[i].campaignDayMillis() == 2, "replay input metadata");
            byte[] encoded = SystemPackageTopology.encode(inputs[i], SystemPackageTopology.DEFAULT);
            byte[] hash = SystemPackageTopology.digest(inputs[i], SystemPackageTopology.DEFAULT);
            images[i] = new Object[] {
                SystemPackageTopology.decode(encoded, SystemPackageTopology.DEFAULT), encoded.clone(), hash.clone()
            };
        }
        return images;
    }

    private static void replayState(SystemPublicationHistory history, int records, int canonicalBytes, int latestIndex) {
        check(history.universe().equals(id(0x11)), "replay namespace");
        check(history.limits().equals(REPLAY_LIMITS), "replay isolated limits");
        check(history.keysLen() == 1, "replay one key");
        check(history.records().size() == records, "replay record count");
        check(history.canonicalBytesLen() == canonicalBytes, "replay canonical byte count");
        assertRecord(history.records().get(0), Kind.PUBLISHED_VERSION, 1, 1, A1_SHA, bytes(A1_HEX));
        if (records >= 2) assertRecord(history.records().get(1), Kind.PUBLISHED_VERSION, 2, 2, A2_SHA, bytes(A2_HEX));
        if (records >= 3) assertRecord(history.records().get(2), Kind.TOMBSTONE, 3, 2, A2_SHA, null);
        if (records == 4) assertRecord(history.records().get(3), Kind.PUBLISHED_VERSION, 4, 4, A4_SHA, bytes(A4_HEX));
        latest(history, latestIndex);
    }

    private static void retainedReplayRows(SystemPublicationHistory history, List<PublicationRecord> rows,
            Object[][] images, PublicationRecord tombstone, Object[] tombstoneImage) {
        check(rows.size() == 3, "pre-P4 list is not a live view");
        check(Arrays.deepEquals(images, replayRecordImages(rows)), "old three-row snapshot remains exact");
        check(Arrays.deepEquals(images, replayRecordImages(history.records().subList(0, 3))), "current prefix preserves old rows");
        assertRecord(tombstone, Kind.TOMBSTONE, 3, 2, A2_SHA, null);
        check(Arrays.deepEquals(tombstoneImage, replayRecordImage(tombstone)), "owned T3 image unchanged");
        check(Arrays.deepEquals(tombstoneImage, replayRecordImage(history.records().get(2))), "current T3 retains original reference");
    }

    private static void historicalReplayPreservesLatestAndTombstoneReferenceAcrossRepublish() {
        byte[][] fixed = {bytes(A1_HEX), bytes(A2_HEX), bytes(A4_HEX)};
        Topology[] inputs = {
            SystemPackageTopology.decode(fixed[0], SystemPackageTopology.DEFAULT),
            SystemPackageTopology.decode(fixed[1], SystemPackageTopology.DEFAULT),
            SystemPackageTopology.decode(fixed[2], SystemPackageTopology.DEFAULT)
        };
        Topology[] originals = {
            SystemPackageTopology.decode(bytes(A1_HEX), SystemPackageTopology.DEFAULT),
            SystemPackageTopology.decode(bytes(A2_HEX), SystemPackageTopology.DEFAULT),
            SystemPackageTopology.decode(bytes(A4_HEX), SystemPackageTopology.DEFAULT)
        };
        replayInputImages(inputs, originals, fixed);
        SystemPublicationHistory history = new SystemPublicationHistory(id(0x11), REPLAY_LIMITS);
        check(history.appendPublication(inputs[0], SystemPackageTopology.DEFAULT) == Outcome.PUBLICATION_APPENDED, "replay append P1");
        replayState(history, 1, 199, 0);
        replayInputImages(inputs, originals, fixed);
        check(history.appendPublication(inputs[1], SystemPackageTopology.DEFAULT) == Outcome.PUBLICATION_APPENDED, "replay append P2");
        replayState(history, 2, 398, 1);
        replayInputImages(inputs, originals, fixed);
        check(history.appendTombstone(id(0x22), id(0x33), id(0x44), 3) == Outcome.TOMBSTONE_APPENDED, "replay append T3");
        replayState(history, 3, 398, 2);
        replayInputImages(inputs, originals, fixed);
        {
            Object[] before = replayHistoryImage(history);
            List<PublicationRecord> beforeRows = history.records();
            Object[][] beforeInputs = replayInputImages(inputs, originals, fixed);
            check(history.appendPublication(inputs[0], SystemPackageTopology.DEFAULT) == Outcome.UNCHANGED, "historical P1 before republish");
            check(Arrays.deepEquals(before, replayHistoryImage(history)), "whole API-visible replay history unchanged");
            check(history.records().equals(beforeRows), "immutable replay list value unchanged");
            check(Arrays.deepEquals(beforeInputs, replayInputImages(inputs, originals, fixed)), "all replay input images unchanged");
            replayState(history, 3, 398, 2);
        }
        {
            Object[] before = replayHistoryImage(history);
            List<PublicationRecord> beforeRows = history.records();
            Object[][] beforeInputs = replayInputImages(inputs, originals, fixed);
            check(history.appendPublication(inputs[1], SystemPackageTopology.DEFAULT) == Outcome.UNCHANGED, "historical P2 before republish");
            check(Arrays.deepEquals(before, replayHistoryImage(history)), "whole API-visible replay history unchanged");
            check(history.records().equals(beforeRows), "immutable replay list value unchanged");
            check(Arrays.deepEquals(beforeInputs, replayInputImages(inputs, originals, fixed)), "all replay input images unchanged");
            replayState(history, 3, 398, 2);
        }
        {
            Object[] before = replayHistoryImage(history);
            List<PublicationRecord> beforeRows = history.records();
            Object[][] beforeInputs = replayInputImages(inputs, originals, fixed);
            check(history.appendTombstone(id(0x22), id(0x33), id(0x44), 3) == Outcome.UNCHANGED, "historical T3 before republish");
            check(Arrays.deepEquals(before, replayHistoryImage(history)), "whole API-visible replay history unchanged");
            check(history.records().equals(beforeRows), "immutable replay list value unchanged");
            check(Arrays.deepEquals(beforeInputs, replayInputImages(inputs, originals, fixed)), "all replay input images unchanged");
            replayState(history, 3, 398, 2);
        }
        List<PublicationRecord> oldRows = history.records();
        Object[][] oldImages = replayRecordImages(oldRows);
        PublicationRecord oldTombstone = oldRows.get(2);
        Object[] oldTombstoneImage = replayRecordImage(oldTombstone);
        unmodifiable(oldRows);
        check(history.appendPublication(inputs[2], SystemPackageTopology.DEFAULT) == Outcome.PUBLICATION_APPENDED, "replay append P4");
        replayState(history, 4, 597, 3);
        replayInputImages(inputs, originals, fixed);
        retainedReplayRows(history, oldRows, oldImages, oldTombstone, oldTombstoneImage);
        check(history.records().size() == history.limits().records(), "replay full record quota");
        check(history.canonicalBytesLen() == history.limits().canonicalBytes(), "replay full canonical byte quota");
        {
            Object[] before = replayHistoryImage(history);
            List<PublicationRecord> beforeRows = history.records();
            Object[][] beforeInputs = replayInputImages(inputs, originals, fixed);
            check(history.appendTombstone(id(0x22), id(0x33), id(0x44), 3) == Outcome.UNCHANGED, "historical T3 after republish at full quota");
            check(Arrays.deepEquals(before, replayHistoryImage(history)), "whole API-visible replay history unchanged");
            check(history.records().equals(beforeRows), "immutable replay list value unchanged");
            check(Arrays.deepEquals(beforeInputs, replayInputImages(inputs, originals, fixed)), "all replay input images unchanged");
            replayState(history, 4, 597, 3);
            retainedReplayRows(history, oldRows, oldImages, oldTombstone, oldTombstoneImage);
        }
        {
            Object[] before = replayHistoryImage(history);
            List<PublicationRecord> beforeRows = history.records();
            Object[][] beforeInputs = replayInputImages(inputs, originals, fixed);
            check(history.appendPublication(inputs[0], SystemPackageTopology.DEFAULT) == Outcome.UNCHANGED, "historical P1 after republish at full quota");
            check(Arrays.deepEquals(before, replayHistoryImage(history)), "whole API-visible replay history unchanged");
            check(history.records().equals(beforeRows), "immutable replay list value unchanged");
            check(Arrays.deepEquals(beforeInputs, replayInputImages(inputs, originals, fixed)), "all replay input images unchanged");
            replayState(history, 4, 597, 3);
            retainedReplayRows(history, oldRows, oldImages, oldTombstone, oldTombstoneImage);
        }
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        immutableVersionsAndTombstonePreserveCanonicalHistory();
        historicalReplayPreservesLatestAndTombstoneReferenceAcrossRepublish();
        System.out.println("NF-SPT-1 Java immutable versions/tombstone/ownership PASS; data only");
        System.out.println("NF-SPT-1 Java historical replay/republish/frozen tombstone reference PASS; data only");
    }
}
