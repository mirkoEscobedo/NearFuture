package nf.nex.reference;

public record PeaceGates(boolean hasEnemies, boolean pirateFaction, boolean allowPirateInvasions,
        float daysSinceLastWar, float minimumWarInterval, boolean recentWar, boolean canCeasefire,
        boolean commissionedTarget, boolean offensiveBlocked, boolean offensiveFactsProvided) {
    public PeaceGates {
        ReferenceBounds.finite(daysSinceLastWar, minimumWarInterval);
        if (Float.floatToRawIntBits(minimumWarInterval) != Float.floatToRawIntBits(30f)) {
            throw new IllegalArgumentException("Unrecognized pinned war interval");
        }
    }
}
