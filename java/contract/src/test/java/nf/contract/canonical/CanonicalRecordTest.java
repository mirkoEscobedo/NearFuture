package nf.contract.canonical;

import java.util.List;
import nf.contract.canonical.Records.Document;

public final class CanonicalRecordTest {
    private CanonicalRecordTest() { }
    public static void main(String[] args) {
        Document required = new Document(6, 2, List.of(new Records.SetValues(List.of(999L)), new Records.SetValues(List.of())));
        try {
            RecordCodec.encode(required);
            throw new AssertionError("Unknown required capability must fail closed");
        } catch (Validation expected) {
            if (expected.code() != Validation.Code.UNKNOWN_REQUIRED_SEMANTIC) throw new AssertionError(expected.code());
        }
        System.out.println("PASS: unknown required semantics reject");
    }
}
