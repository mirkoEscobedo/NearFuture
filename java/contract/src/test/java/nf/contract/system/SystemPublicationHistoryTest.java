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

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        immutableVersionsAndTombstonePreserveCanonicalHistory();
        System.out.println("NF-SPT-1 Java immutable versions/tombstone/ownership PASS; data only");
    }
}
