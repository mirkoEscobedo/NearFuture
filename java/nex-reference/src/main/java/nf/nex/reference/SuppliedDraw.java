package nf.nex.reference;

public record SuppliedDraw(String purpose, long doubleBits) {
    public SuppliedDraw {
        double value = Double.longBitsToDouble(doubleBits);
        if (purpose == null || !Double.isFinite(value) || value < 0 || value >= 1) {
            throw new IllegalArgumentException("Invalid Math.random draw");
        }
    }
    public double value() { return Double.longBitsToDouble(doubleBits); }
}
