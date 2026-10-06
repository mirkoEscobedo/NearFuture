package nf.contract.canonical;

import java.util.List;

/** Immutable values for the closed semantic record registry. */
public final class Records {
    private Records() { }
    public sealed interface Value permits Id, Digest, U8, U32, U64, Bool, Text, Bytes, OptionalU64,
            Revisions, SetValues, Nested, Documents, Outcome { }
    public record Id(byte[] value) implements Value {
        public Id { if (value.length != 16) throw new Validation(Validation.Code.INVALID_VALUE); value = value.clone(); }
        @Override public byte[] value() { return value.clone(); }
    }
    public record Digest(byte[] value) implements Value {
        public Digest { if (value.length != 32) throw new Validation(Validation.Code.INVALID_VALUE); value = value.clone(); }
        @Override public byte[] value() { return value.clone(); }
    }
    public record U8(int value) implements Value {
        public U8 { if (value < 0 || value > 255) throw new Validation(Validation.Code.INVALID_VALUE); }
    }
    public record U32(long value) implements Value {
        public U32 { if (value < 0 || value > 0xffff_ffffL) throw new Validation(Validation.Code.INVALID_VALUE); }
    }
    public record U64(long rawBits) implements Value { }
    public record Bool(boolean value) implements Value { }
    public record Text(String value) implements Value { }
    public record Bytes(byte[] value) implements Value {
        public Bytes { if (value.length > Limits.DEFAULT.bytesBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED); value = value.clone(); }
        @Override public byte[] value() { return value.clone(); }
    }
    public record OptionalU64(Long rawBits) implements Value { }
    public record Revision(Id aggregateId, long rawBits) { }
    public record Revisions(List<Revision> values) implements Value {
        public Revisions { values = List.copyOf(values); }
    }
    public record SetValues(List<Long> values) implements Value {
        public SetValues { values = List.copyOf(values); }
    }
    public record Nested(Document value) implements Value { }
    public record Documents(List<Document> values) implements Value {
        public Documents { values = List.copyOf(values); }
    }
    public record Outcome(Document value) implements Value { }
    public record Document(int domain, int kind, List<Value> fields) {
        public Document { fields = List.copyOf(fields); }
    }
}
