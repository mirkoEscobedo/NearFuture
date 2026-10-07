package nf.nex.reference;

public record PriorityEntry(String id, Kind kind, int floatBits) {
    public enum Kind { FLAT, PERCENT, MULTIPLIER }
    public PriorityEntry {
        if (id == null || id.isEmpty() || id.length() > 128 || kind == null || !Float.isFinite(Float.intBitsToFloat(floatBits))) {
            throw new IllegalArgumentException("Unsupported priority entry");
        }
    }
    public static PriorityEntry of(String id, Kind kind, float value) {
        return new PriorityEntry(id, kind, Float.floatToRawIntBits(value));
    }
}
