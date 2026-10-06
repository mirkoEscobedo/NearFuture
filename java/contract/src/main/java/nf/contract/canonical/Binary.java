package nf.contract.canonical;

final class Binary {
    private Binary() { }
    static final byte[] PREFIX = {0x4e, 0x46, 0x2d, 0x43, 0x41, 0x4e, 0x4f, 0x4e, 0x2d, 0x31, 0};
    static final class Reader {
        private final byte[] input;
        final Limits limits;
        private int offset;
        private long allocated;
        private int items;
        private final int maximumItems;
        Reader(byte[] input, Limits limits) { this(input, limits, limits.totalItems()); }
        Reader(byte[] input, Limits limits, int maximumItems) {
            this.maximumItems = maximumItems;
            if (input.length > limits.documentBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
            this.limits = limits;
            allocate(input.length);
            this.input = input.clone();
        }
        void allocate(long bytes) {
            allocated += bytes;
            if (allocated > limits.allocationBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        }
        void require(int count) {
            if (count < 0 || count > input.length - offset) throw new Validation(Validation.Code.TRUNCATED);
        }
        long number(int count) {
            require(count);
            long value = 0;
            for (int i = 0; i < count; i++) value |= (long) (input[offset++] & 255) << (i * 8);
            return value;
        }
        int count(int maximum) {
            long count = number(4);
            if (count > maximum) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
            return (int) count;
        }
        int entries(int maximum) {
            int count = count(maximum);
            items += count;
            if (items > maximumItems) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
            allocate(count * 64L);
            return count;
        }
        byte[] raw(int count) {
            require(count);
            allocate(count * 2L + 24);
            byte[] bytes = java.util.Arrays.copyOfRange(input, offset, offset + count);
            offset += count;
            return bytes;
        }
        byte[] field(int maximum) { return raw(count(maximum)); }
        int presence() {
            int value = (int) number(1);
            if (value > 1) throw new Validation(Validation.Code.INVALID_PRESENCE);
            return value;
        }
        int[] header() {
            require(PREFIX.length);
            for (byte value : PREFIX) if ((byte) number(1) != value) throw new Validation(Validation.Code.UNKNOWN_RECORD);
            int domain = (int) number(2), kind = (int) number(2);
            if (number(2) != 1) throw new Validation(Validation.Code.UNKNOWN_RECORD);
            allocate(24);
            return new int[] {domain, kind};
        }
        void header(int domain, int kind) {
            require(PREFIX.length);
            for (byte value : PREFIX) if ((byte) number(1) != value) throw new Validation(Validation.Code.UNKNOWN_RECORD);
            if (number(2) != domain || number(2) != kind || number(2) != 1) throw new Validation(Validation.Code.UNKNOWN_RECORD);
        }
        boolean finished() { return offset == input.length; }
        void finish() { if (!finished()) throw new Validation(Validation.Code.TRAILING_BYTES); }
    }
    static final class Writer {
        private final byte[] output;
        private int offset;
        Writer(int size) { output = new byte[size]; }
        void number(long value, int bytes) {
            for (int i = 0; i < bytes; i++) output[offset++] = (byte) (value >>> (i * 8));
        }
        void raw(byte[] value) { System.arraycopy(value, 0, output, offset, value.length); offset += value.length; }
        void field(byte[] value) { number(value.length, 4); raw(value); }
        void header(int domain, int kind) { raw(PREFIX); number(domain, 2); number(kind, 2); number(1, 2); }
        byte[] finish() {
            if (offset != output.length) throw new Validation(Validation.Code.INVALID_VALUE);
            return output;
        }
    }
}
