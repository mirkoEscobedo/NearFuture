package nf.nex.reference;

import java.util.List;

public record PeaceInput(String faction, String enemy, boolean enemyIsPlayer, boolean treatyRelationEligible,
        float ownWeariness, float enemyWeariness, float ownEventDisposition, float enemyEventDisposition,
        PeaceGates gates, PeaceRules rules, List<SuppliedDraw> draws) {
    public PeaceInput {
        ReferenceBounds.finite(ownWeariness, enemyWeariness, ownEventDisposition, enemyEventDisposition);
        ReferenceBounds.count(draws, 2);
        if (faction == null || enemy == null || faction.isEmpty() || enemy.isEmpty()
                || faction.length() > 128 || enemy.length() > 128 || faction.equals(enemy) || gates == null || rules == null) {
            throw new IllegalArgumentException("Missing peace scope/facts");
        }
        draws = List.copyOf(draws);
    }
}
