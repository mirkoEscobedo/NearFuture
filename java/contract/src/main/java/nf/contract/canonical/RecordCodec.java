package nf.contract.canonical;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import nf.contract.canonical.Records.*;

/** Closed semantic codec. Decoding establishes data validity, never commit authority. */
public final class RecordCodec {
    private RecordCodec() { }
    private static final class Budget {
        final Limits limits;
        int items;
        Budget(Limits limits) { this.limits = limits; }
        void entries(int count, int maximum) {
            items += count;
            if (count > maximum || items > limits.totalItems()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        }
        void depth(int depth) { if (depth > limits.nestingDepth()) throw new Validation(Validation.Code.LIMIT_EXCEEDED); }
    }
    public static byte[] encode(Document document) { return encode(document, Limits.DEFAULT); }
    public static byte[] encode(Document document, Limits limits) {
        int size = measure(document, new Budget(limits), 1);
        Binary.Writer writer = new Binary.Writer(size);
        write(document, writer);
        return writer.finish();
    }
    private static int measure(Document document, Budget budget, int depth) {
        budget.depth(depth);
        Registry.Field[] fields = Registry.fields(document.domain(), document.kind());
        if (fields.length != document.fields().size()) throw new Validation(Validation.Code.INVALID_VALUE);
        long size = 17;
        for (int i = 0; i < fields.length; i++) {
            size += measureValue(document.fields().get(i), fields[i], budget, depth);
            if (size > budget.limits.documentBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        }
        semantics(document, budget);
        return (int) size;
    }
    private static int measureValue(Value value, Registry.Field field, Budget budget, int depth) {
        Limits limits = budget.limits;
        if (value instanceof Id && field.kind() == Registry.Kind.ID) return 16;
        if (value instanceof Digest && field.kind() == Registry.Kind.DIGEST) return 32;
        if (value instanceof U8 && field.kind() == Registry.Kind.U8) return 1;
        if (value instanceof U32 && field.kind() == Registry.Kind.U32) return 4;
        if (value instanceof U64 && field.kind() == Registry.Kind.U64) return 8;
        if (value instanceof Bool && field.kind() == Registry.Kind.BOOL) return 1;
        if (value instanceof Text text && field.kind() == Registry.Kind.TEXT) return 4 + Canonical.utf8(text.value(), limits.textBytes()).length;
        if (value instanceof Bytes bytes && field.kind() == Registry.Kind.BYTES) {
            if (bytes.value().length > limits.bytesBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
            return 4 + bytes.value().length;
        }
        if (value instanceof OptionalU64 optional && field.kind() == Registry.Kind.OPTIONAL64) return optional.rawBits() == null ? 1 : 9;
        if (value instanceof Revisions revisions && field.kind() == Registry.Kind.REVISIONS) {
            budget.entries(revisions.values().size(), limits.collectionItems());
            byte[] previous = null;
            for (Revision revision : revisions.values()) { ordered(previous, revision.aggregateId().value()); previous = revision.aggregateId().value(); }
            return 4 + revisions.values().size() * 24;
        }
        if (value instanceof SetValues set && field.kind() == Registry.Kind.SET) {
            budget.entries(set.values().size(), 64);
            long previous = -1;
            for (long entry : set.values()) {
                if (entry <= 0 || entry > 0xffff_ffffL || entry <= previous) throw new Validation(Validation.Code.INVALID_VALUE);
                previous = entry;
            }
            return 4 + set.values().size() * 4;
        }
        if (value instanceof Nested nested && field.kind() == Registry.Kind.NESTED) {
            expected(nested.value(), field.domain(), field.type());
            return measure(nested.value(), budget, depth + 1);
        }
        if (value instanceof Documents documents && field.kind() == Registry.Kind.DOCUMENTS) {
            budget.entries(documents.values().size(), limits.collectionItems());
            long size = 4;
            byte[] previous = null;
            for (Document nested : documents.values()) {
                expected(nested, field.domain(), field.type());
                size += measure(nested, budget, depth + 1);
                if (size > limits.documentBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
                if (field.sortField() >= 0) { byte[] key = ((Id) nested.fields().get(field.sortField())).value(); ordered(previous, key); previous = key; }
            }
            return (int) size;
        }
        if (value instanceof Outcome outcome && field.kind() == Registry.Kind.OUTCOME) {
            if (outcome.value() == null) return 1;
            if (outcome.value().domain() != 6 || (outcome.value().kind() != 1 && outcome.value().kind() != 10)) throw new Validation(Validation.Code.INVALID_VALUE);
            return 1 + measure(outcome.value(), budget, depth + 1);
        }
        throw new Validation(Validation.Code.INVALID_VALUE);
    }
    private static void expected(Document document, int domain, int kind) {
        if (document.domain() != domain || document.kind() != kind) throw new Validation(Validation.Code.UNKNOWN_RECORD);
    }
    private static void ordered(byte[] previous, byte[] key) {
        if (previous != null && Arrays.compareUnsigned(previous, key) >= 0) throw new Validation(Arrays.equals(previous, key) ? Validation.Code.DUPLICATE_KEY : Validation.Code.KEY_ORDER);
    }
    private static void write(Document document, Binary.Writer writer) {
        writer.header(document.domain(), document.kind());
        for (Value value : document.fields()) {
            if (value instanceof Id id) writer.raw(id.value());
            else if (value instanceof Digest digest) writer.raw(digest.value());
            else if (value instanceof U8 number) writer.number(number.value(), 1);
            else if (value instanceof U32 number) writer.number(number.value(), 4);
            else if (value instanceof U64 number) writer.number(number.rawBits(), 8);
            else if (value instanceof Bool bool) writer.number(bool.value() ? 1 : 0, 1);
            else if (value instanceof Text text) writer.field(Canonical.utf8(text.value(), Limits.DEFAULT.textBytes()));
            else if (value instanceof Bytes bytes) writer.field(bytes.value());
            else if (value instanceof OptionalU64 optional) { writer.number(optional.rawBits() == null ? 0 : 1, 1); if (optional.rawBits() != null) writer.number(optional.rawBits(), 8); }
            else if (value instanceof Revisions revisions) { writer.number(revisions.values().size(), 4); for (Revision revision : revisions.values()) { writer.raw(revision.aggregateId().value()); writer.number(revision.rawBits(), 8); } }
            else if (value instanceof SetValues set) { writer.number(set.values().size(), 4); for (long entry : set.values()) writer.number(entry, 4); }
            else if (value instanceof Nested nested) write(nested.value(), writer);
            else if (value instanceof Documents documents) { writer.number(documents.values().size(), 4); for (Document nested : documents.values()) write(nested, writer); }
            else if (value instanceof Outcome outcome) { writer.number(outcome.value() == null ? 0 : outcome.value().kind() == 1 ? 1 : 2, 1); if (outcome.value() != null) write(outcome.value(), writer); }
            else throw new Validation(Validation.Code.INVALID_VALUE);
        }
    }
    private static void semantics(Document document, Budget budget) {
        Limits limits = budget.limits;
        List<Value> fields = document.fields();
        if (document.domain() == 6 && document.kind() == 1) {
            if (((U32) fields.get(0)).value() != 1) throw new Validation(Validation.Code.UNKNOWN_REQUIRED_SEMANTIC);
            Canonical.Profile profile = Canonical.decodeProfile(((Bytes) fields.get(1)).value(), limits, limits.totalItems() - budget.items);
            budget.entries(profile.integers().size(), limits.collectionItems());
            budget.entries(profile.counters().size(), limits.collectionItems());
        } else if (document.domain() == 6 && document.kind() == 2) {
            for (Value value : fields) for (long id : ((SetValues) value).values()) if (id != 1) throw new Validation(Validation.Code.UNKNOWN_REQUIRED_SEMANTIC);
        } else if (document.domain() == 1 && document.kind() == 2) {
            Document binding = ((Nested) fields.get(0)).value();
            if (((U32) binding.fields().get(5)).value() != 1) throw new Validation(Validation.Code.UNKNOWN_REQUIRED_SEMANTIC);
            byte[] expected = ((Digest) binding.fields().get(6)).value();
            byte[] actual = Canonical.sha256(encode(((Nested) fields.get(2)).value(), limits));
            if (!Arrays.equals(expected, actual)) throw new Validation(Validation.Code.DIGEST_MISMATCH);
        } else if (document.domain() == 6 && document.kind() == 8) {
            long phase = ((U32) fields.get(4)).value();
            Document outcome = ((Outcome) fields.get(5)).value();
            Long committed = ((OptionalU64) fields.get(6)).rawBits();
            if (phase < 1 || phase > 5 || ((phase <= 2) && (outcome != null || committed != null))
                    || (phase == 3 && (outcome == null || outcome.kind() != 1 || committed == null))
                    || (phase >= 4 && (outcome == null || outcome.kind() != 10))) throw new Validation(Validation.Code.INVALID_VALUE);
        } else if (document.domain() == 6 && document.kind() == 9) {
            Document status = ((Nested) fields.get(2)).value();
            if (!Arrays.equals(((Id) fields.get(0)).value(), ((Id) status.fields().get(0)).value())
                    || !Arrays.equals(((Digest) fields.get(1)).value(), ((Digest) status.fields().get(3)).value())) {
                throw new Validation(Validation.Code.INVALID_VALUE);
            }
        } else if (document.domain() == 6 && document.kind() == 10) {
            long code = ((U32) fields.get(0)).value();
            if (code < 1 || code > 10) throw new Validation(Validation.Code.INVALID_VALUE);
            Canonical.utf8(((Text) fields.get(1)).value(), Math.min(512, limits.textBytes()));
        } else if (document.domain() == 2) {
            byte[] history = ((Id) fields.get(1)).value();
            for (Document status : ((Documents) fields.get(10)).values()) historyMatches(history, status);
            for (Document dedup : ((Documents) fields.get(11)).values()) historyMatches(history, ((Nested) dedup.fields().get(2)).value());
        } else if (document.domain() == 4) {
            byte[] history = ((Id) fields.get(0)).value();
            for (Document status : ((Documents) fields.get(6)).values()) historyMatches(history, status);
        } else if ((document.domain() == 6 && document.kind() == 4) || document.domain() == 3) {
            if (((U32) fields.get(document.domain() == 3 ? 2 : 1)).value() != 1) throw new Validation(Validation.Code.INVALID_VALUE);
        }
    }
    private static void historyMatches(byte[] history, Document status) {
        if (!Arrays.equals(history, ((Id) status.fields().get(2)).value())) throw new Validation(Validation.Code.INVALID_VALUE);
    }
    public static Document decode(byte[] input) { return decode(input, Limits.DEFAULT); }
    public static Document decode(byte[] input, Limits limits) {
        Binary.Reader reader = new Binary.Reader(input, limits);
        Document document = read(reader, 1);
        reader.finish();
        measure(document, new Budget(limits), 1);
        return document;
    }
    private static Document read(Binary.Reader reader, int depth) {
        if (depth > reader.limits.nestingDepth()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        int[] header = reader.header();
        Registry.Field[] fields = Registry.fields(header[0], header[1]);
        reader.allocate(fields.length * 64L + 64);
        ArrayList<Value> values = new ArrayList<>(fields.length);
        for (Registry.Field field : fields) values.add(readValue(reader, field, depth));
        return new Document(header[0], header[1], values);
    }
    private static Value readValue(Binary.Reader reader, Registry.Field field, int depth) {
        return switch (field.kind()) {
            case ID -> new Id(reader.raw(16));
            case DIGEST -> new Digest(reader.raw(32));
            case U8 -> new U8((int) reader.number(1));
            case U32 -> new U32(reader.number(4));
            case U64 -> new U64(reader.number(8));
            case BOOL -> { int value = reader.presence(); yield new Bool(value != 0); }
            case TEXT -> { byte[] bytes = reader.field(reader.limits.textBytes()); reader.allocate(bytes.length * 2L); yield new Text(Canonical.text(bytes)); }
            case BYTES -> new Bytes(reader.field(reader.limits.bytesBytes()));
            case OPTIONAL64 -> new OptionalU64(reader.presence() == 0 ? null : reader.number(8));
            case REVISIONS -> {
                int count = reader.entries(reader.limits.collectionItems()); reader.require(count * 24);
                ArrayList<Revision> values = new ArrayList<>(count);
                for (int i = 0; i < count; i++) values.add(new Revision(new Id(reader.raw(16)), reader.number(8)));
                yield new Revisions(values);
            }
            case SET -> {
                int count = reader.entries(64); reader.require(count * 4);
                ArrayList<Long> values = new ArrayList<>(count);
                for (int i = 0; i < count; i++) values.add(reader.number(4));
                yield new SetValues(values);
            }
            case NESTED -> { Document nested = read(reader, depth + 1); expected(nested, field.domain(), field.type()); yield new Nested(nested); }
            case DOCUMENTS -> {
                int count = reader.entries(reader.limits.collectionItems());
                ArrayList<Document> values = new ArrayList<>(count);
                for (int i = 0; i < count; i++) { Document nested = read(reader, depth + 1); expected(nested, field.domain(), field.type()); values.add(nested); }
                yield new Documents(values);
            }
            case OUTCOME -> {
                int tag = (int) reader.number(1);
                if (tag > 2) throw new Validation(Validation.Code.INVALID_VALUE);
                Document outcome = tag == 0 ? null : read(reader, depth + 1);
                if (outcome != null) expected(outcome, 6, tag == 1 ? 1 : 10);
                yield new Outcome(outcome);
            }
        };
    }
}
