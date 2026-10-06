package nf.contract.canonical;

import java.util.List;

public final class CanonicalSemanticLimitsTest {
    private CanonicalSemanticLimitsTest() { }
    public static void main(String[] args) {
        try { new Limits(1048577, 4096, 262144, 4096, 16384, 32, 4194304); throw new AssertionError("Caller raised document hard cap"); } catch (Validation expected) { }
        try { new Limits(1048576, 4096, 262144, 4097, 16384, 32, 4194304); throw new AssertionError("Caller raised collection hard cap"); } catch (Validation expected) { }
        Records.Document zeroSemantic = new Records.Document(6, 10, List.of(new Records.U32(4), new Records.Text("unsupported"),
                new Records.SetValues(List.of(0L)), new Records.SetValues(List.of()), new Records.Bool(false)));
        try { RecordCodec.encode(zeroSemantic); throw new AssertionError("Zero semantic selector admitted"); }
        catch (Validation expected) { }
        byte[] body = Canonical.encodeProfile(new Canonical.Profile(null, null, java.util.Collections.nCopies(4096, 0L), List.of()));
        Records.Document payload = new Records.Document(6, 1, List.of(new Records.U32(1), new Records.Bytes(body)));
        Records.Document required = new Records.Document(6, 2, List.of(new Records.SetValues(List.of()), new Records.SetValues(List.of())));
        Records.Document proposal = new Records.Document(3, 1, List.of(new Records.Id(new byte[16]), new Records.Id(new byte[16]),
                new Records.U32(1), new Records.Digest(new byte[32]), new Records.Digest(new byte[32]), new Records.U64(1),
                new Records.Revisions(List.of()), new Records.Revisions(List.of()), new Records.Documents(java.util.Collections.nCopies(5, payload)),
                new Records.Documents(List.of()), new Records.Nested(required)));
        try { RecordCodec.encode(proposal); throw new AssertionError("Nested payloads bypass cumulative 16384-entry cap"); }
        catch (Validation expected) { if (expected.code() != Validation.Code.LIMIT_EXCEEDED) throw new AssertionError(expected.code()); }
        Records.Document binding = GoldenCorpus.BINDINGS.get(0).record();
        Records.Document aliased = new Records.Document(0, 257, binding.fields());
        try { RecordCodec.encode(aliased); throw new AssertionError("Unknown u16 registry pair aliased RequestBinding"); } catch (Validation expected) { }
        byte[] unknown = GoldenCorpus.hex(GoldenCorpus.BINDINGS.get(0).hex());
        unknown[11] = 0; unknown[12] = 0; unknown[13] = 1; unknown[14] = 1;
        try { RecordCodec.decode(unknown); throw new AssertionError("Unknown u16 header pair aliased RequestBinding"); } catch (Validation expected) { }
        Records.Id history = new Records.Id(new byte[16]);
        byte[] foreign = new byte[16]; foreign[0] = 1;
        Records.Document status = new Records.Document(6, 8, List.of(history, history, new Records.Id(foreign),
                new Records.Digest(new byte[32]), new Records.U32(1), new Records.Outcome(null), new Records.OptionalU64(null)));
        Records.Document snapshot = GoldenCorpus.SEMANTIC.stream().filter(f -> f.record().domain() == 2).findFirst().orElseThrow().record();
        java.util.ArrayList<Records.Value> snapshotFields = new java.util.ArrayList<>(snapshot.fields());
        snapshotFields.set(10, new Records.Documents(List.of(status)));
        try { RecordCodec.encode(new Records.Document(2, 1, snapshotFields)); throw new AssertionError("Foreign status history in Snapshot admitted"); } catch (Validation expected) { }
        Records.Document dedup = new Records.Document(6, 9, List.of(history, new Records.Digest(new byte[32]), new Records.Nested(status)));
        snapshotFields.set(10, new Records.Documents(List.of())); snapshotFields.set(11, new Records.Documents(List.of(dedup)));
        try { RecordCodec.encode(new Records.Document(2, 1, snapshotFields)); throw new AssertionError("Foreign dedup history in Snapshot admitted"); } catch (Validation expected) { }
        Records.Document event = GoldenCorpus.SEMANTIC.stream().filter(f -> f.record().domain() == 4).findFirst().orElseThrow().record();
        java.util.ArrayList<Records.Value> eventFields = new java.util.ArrayList<>(event.fields());
        eventFields.set(6, new Records.Documents(List.of(status)));
        try { RecordCodec.encode(new Records.Document(4, 1, eventFields)); throw new AssertionError("Foreign status history in EventBatch admitted"); } catch (Validation expected) { }
        System.out.println("PASS: hard ceilings, exact registry, nonzero selectors, cumulative payload entries, bound histories");
    }
}
