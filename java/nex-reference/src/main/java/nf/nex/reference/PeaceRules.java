package nf.nex.reference;

public record PeaceRules(float minimumWeariness, float divisor, float divisorPerLevel, int playerLevel,
        float eventPeaceMultiplier, float treatyChance, float ceasefireReduction, float treatyReduction) {
    public PeaceRules {
        ReferenceBounds.finite(minimumWeariness, divisor, divisorPerLevel, eventPeaceMultiplier,
                treatyChance, ceasefireReduction, treatyReduction);
        if (Float.floatToRawIntBits(eventPeaceMultiplier) != Float.floatToRawIntBits(40f)
                || Float.floatToRawIntBits(treatyChance) != Float.floatToRawIntBits(0.3f)) {
            throw new IllegalArgumentException("Unrecognized pinned peace constants");
        }
        if (playerLevel < 0 || playerLevel > 10_000) throw new IllegalArgumentException("Unsupported level");
    }
}
