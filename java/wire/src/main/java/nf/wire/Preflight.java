package nf.wire;

import com.google.protobuf.Descriptors.Descriptor;
import com.google.protobuf.Descriptors.FieldDescriptor;
import com.google.protobuf.Descriptors.OneofDescriptor;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;

/** Bounded descriptor-driven guard; maintained generated code still performs actual decoding. */
final class Preflight {
    private Preflight() { }
    static final class Budget {
        long bytes;
        int entries;
        final boolean auth;
        Budget() { this(false); }
        Budget(boolean auth) { this.auth=auth; }
        void charge(long amount) { bytes += amount; if (bytes > (auth ? 4096 : 1_048_576)) throw new WireFailure(WireFailure.Code.LIMIT); }
        void entry() { if (++entries > (auth ? 256 : 16_384)) throw new WireFailure(WireFailure.Code.LIMIT); }
    }
    private static final class Cursor {
        final byte[] bytes;
        final int end;
        int position;
        Cursor(byte[] bytes, int start, int end) { this.bytes = bytes; this.position = start; this.end = end; }
        void require(int count) { if (count < 0 || count > end - position) throw new WireFailure(WireFailure.Code.MALFORMED); }
        long varint() {
            long value = 0;
            for (int i = 0; i < 10; i++) {
                require(1);
                int octet = bytes[position++] & 255;
                if (i == 9 && octet > 1) throw new WireFailure(WireFailure.Code.MALFORMED);
                value |= (long) (octet & 127) << (i * 7);
                if ((octet & 128) == 0) {
                    if (i > 0 && (octet & 127) == 0) throw new WireFailure(WireFailure.Code.MALFORMED);
                    return value;
                }
            }
            throw new WireFailure(WireFailure.Code.MALFORMED);
        }
        int length() { return length(1_048_576); }
        int length(int maximum) {
            long length = varint();
            if (Long.compareUnsigned(length, maximum) > 0) throw new WireFailure(WireFailure.Code.LIMIT);
            int count = (int) length;
            require(count);
            return count;
        }
    }
    static void inspect(byte[] input, Descriptor descriptor) {
        if (input.length > 1_048_576) throw new WireFailure(WireFailure.Code.LIMIT);
        scan(new Cursor(input, 0, input.length), descriptor, 1, new Budget());
    }
    static void inspectAuth(byte[] input, Descriptor descriptor) {
        if (input.length>4096) throw new WireFailure(WireFailure.Code.LIMIT);
        scan(new Cursor(input,0,input.length),descriptor,1,new Budget(true));
    }
    private static int[] authRequired(Descriptor descriptor) {
        return switch (descriptor.getName()) {
            case "LocalAuthEnvelope" -> new int[]{1,2,3,4};
            case "LocalAuthHello" -> new int[]{1,2,3,4,5};
            case "LocalAuthChallenge" -> new int[]{1,2,3};
            case "LocalAuthProof", "LocalAuthAccepted", "RuntimeSession", "AccountId", "DeviceId" -> new int[]{1};
            case "Principal", "ProtocolRange" -> new int[]{1,2};
            case "ResourceLimits" -> new int[]{1,2,3,4,5,6,7,8};
            default -> new int[0];
        };
    }
    private static void scan(Cursor cursor, Descriptor descriptor, int depth, Budget budget) {
        if (depth > (budget.auth ? 8 : 32)) throw new WireFailure(WireFailure.Code.LIMIT);
        budget.charge(256);
        Set<Integer> singular = new HashSet<>();
        Set<OneofDescriptor> oneofs = new HashSet<>();
        Map<Integer, Boolean> repeatedEncoding = new HashMap<>();
        Map<Integer, Integer> repeatedCount = new HashMap<>();
        boolean metadata = descriptor.getName().equals("TransportMetadata");
        while (cursor.position < cursor.end) {
            budget.entry();
            long tag = cursor.varint();
            if (Long.compareUnsigned(tag, 0xffff_ffffL) > 0 || (tag >>> 3) == 0) throw new WireFailure(WireFailure.Code.MALFORMED);
            int number = (int) (tag >>> 3), wire = (int) (tag & 7);
            if (wire == 3 || wire == 4 || wire > 5) throw new WireFailure(WireFailure.Code.MALFORMED);
            FieldDescriptor field = descriptor.findFieldByNumber(number);
            if (field == null) {
                if (!metadata) throw new WireFailure(WireFailure.Code.UNKNOWN_FIELD);
                skip(cursor, wire, budget);
                continue;
            }
            if (!field.isRepeated()) {
                if (!singular.add(number)) throw new WireFailure(WireFailure.Code.DUPLICATE);
                if (field.getContainingOneof() != null && !oneofs.add(field.getContainingOneof())) throw new WireFailure(WireFailure.Code.DUPLICATE);
            }
            int expected = wireType(field);
            boolean packed = field.isRepeated() && field.isPackable() && wire == 2;
            if (wire != expected && !packed) throw new WireFailure(WireFailure.Code.MALFORMED);
            if (field.isRepeated()) {
                Boolean previous = repeatedEncoding.putIfAbsent(number, packed);
                if (previous != null && (previous != packed || packed)) throw new WireFailure(WireFailure.Code.DUPLICATE);
            }
            if (packed) {
                int count = cursor.length();
                int end = cursor.position + count;
                Cursor values = new Cursor(cursor.bytes, cursor.position, end);
                while (values.position < end) {
                    budget.entry();
                    repeat(field, repeatedCount, budget);
                    scalar(values, field, expected, budget);
                }
                cursor.position = end;
            } else {
                if (field.isRepeated()) repeat(field, repeatedCount, budget);
                if (expected == 2) {
                    int maximum = field.getType() == FieldDescriptor.Type.MESSAGE ? 1_048_576
                            : field.getType() == FieldDescriptor.Type.STRING ? (descriptor.getName().equals("BoundedError") ? 512 : 4096) : 262_144;
                    if (field.getName().equals("signature") || field.getName().equals("multihash")) maximum = 128;
                    if (budget.auth) maximum=Math.min(maximum,4096);
                    if (descriptor.getFullName().startsWith("nearfuture.ipc.v1.") && field.getType()==FieldDescriptor.Type.BYTES) maximum=32;
                    int count = cursor.length(maximum);
                    if (field.getType() == FieldDescriptor.Type.MESSAGE) {
                        Cursor nested = new Cursor(cursor.bytes, cursor.position, cursor.position + count);
                        scan(nested, field.getMessageType(), depth + 1, budget);
                    } else {
                        int cap = field.getType() == FieldDescriptor.Type.STRING ? (descriptor.getName().equals("BoundedError") ? 512 : 4096) : 262_144;
                        if (field.getName().equals("signature") || field.getName().equals("multihash")) cap = 128;
                        if (count > cap) throw new WireFailure(WireFailure.Code.LIMIT);
                        budget.charge(count);
                        if (field.getType() == FieldDescriptor.Type.STRING) text(cursor.bytes, cursor.position, count);
                    }
                    cursor.position += count;
                } else scalar(cursor, field, expected, budget);
            }
        }
        if (budget.auth) for (int required:authRequired(descriptor)) if (!singular.contains(required)) throw new WireFailure(WireFailure.Code.SEMANTIC);
    }
    private static void repeat(FieldDescriptor field, Map<Integer, Integer> counts, Budget budget) {
        int count = counts.merge(field.getNumber(), 1, Integer::sum);
        int maximum = field.getContainingType().getName().equals("RequiredSemantics")
                || field.getName().equals("optional_capability_ids")
                || field.getName().startsWith("unsupported_") ? 64 : 4096;
        if (budget.auth) maximum=Math.min(maximum,64);
        if (count > maximum) throw new WireFailure(WireFailure.Code.LIMIT);
    }
    private static int wireType(FieldDescriptor field) {
        return switch (field.getType()) {
            case FIXED64, SFIXED64, DOUBLE -> 1;
            case FIXED32, SFIXED32, FLOAT -> 5;
            case MESSAGE, STRING, BYTES -> 2;
            case GROUP -> throw new WireFailure(WireFailure.Code.MALFORMED);
            default -> 0;
        };
    }
    private static void scalar(Cursor cursor, FieldDescriptor field, int wire, Budget budget) {
        budget.charge(8);
        if (wire == 1 || wire == 5) { int bytes = wire == 1 ? 8 : 4; cursor.require(bytes); cursor.position += bytes; return; }
        long value = cursor.varint();
        if ((field.getType() == FieldDescriptor.Type.UINT32 || field.getType() == FieldDescriptor.Type.ENUM)
                && Long.compareUnsigned(value, 0xffff_ffffL) > 0) throw new WireFailure(WireFailure.Code.MALFORMED);
        if (field.getType() == FieldDescriptor.Type.BOOL && value != 0 && value != 1) throw new WireFailure(WireFailure.Code.MALFORMED);
    }
    private static void skip(Cursor cursor, int wire, Budget budget) {
        if (wire == 0) { cursor.varint(); budget.charge(8); }
        else if (wire == 1 || wire == 5) { int bytes = wire == 1 ? 8 : 4; cursor.require(bytes); cursor.position += bytes; budget.charge(bytes); }
        else if (wire == 2) { int bytes = cursor.length(262_144); cursor.position += bytes; budget.charge(bytes); }
        else throw new WireFailure(WireFailure.Code.MALFORMED);
    }
    private static void text(byte[] input, int offset, int count) {
        byte[] bytes = Arrays.copyOfRange(input, offset, offset + count);
        String value = new String(bytes, StandardCharsets.UTF_8);
        if (!Arrays.equals(bytes, value.getBytes(StandardCharsets.UTF_8))) throw new WireFailure(WireFailure.Code.MALFORMED);
    }
}
