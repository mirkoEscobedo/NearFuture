package nf.nex.reference;

import java.util.List;

/** Requested copied-fact facets only; Java retains authority and this performs no game effects. */
public final class FirstProposalReference {
    private FirstProposalReference() { }
    public enum Operation { GENERATE, UPDATE, VALID }
    public record Eligibility(boolean enabled, boolean concernCanMakePeace,
            Boolean targetHostile, boolean factionDisabled) { }
    public record Input(Operation operation, WarInput concern, Eligibility eligibility,
            PeaceInput selected) {
        public Input {
            if (operation == null || concern == null || eligibility == null || selected == null) {
                throw new IllegalArgumentException("Missing copied facets");
            }
            // Diagnostic finite/positive-minimum domain, not a newly invented Nex refusal.
            ReferenceBounds.finite(concern.weariness(), concern.minimumForPeace());
            if (concern.minimumForPeace() <= 0
                    || Float.floatToRawIntBits(concern.weariness()) != Float.floatToRawIntBits(selected.ownWeariness())
                    || Float.floatToRawIntBits(concern.minimumForPeace()) != Float.floatToRawIntBits(selected.rules().minimumWeariness())) {
                throw new IllegalArgumentException("Inconsistent copied facets");
            }
        }
    }
    public record Output(WarResult concern, boolean valid, boolean eligible,
            PriorityEntry additionalPriority, PeaceResult selected) { }
    public static Output evaluate(Input input) {
        WarResult concern = switch (input.operation()) {
            case GENERATE -> WarWearinessReference.generate(input.concern());
            case UPDATE -> WarWearinessReference.update(input.concern());
            case VALID -> new WarResult(false, input.concern().alreadyEnded(), false,
                    input.concern().priority().existingPriority(), List.of());
        };
        Eligibility eligibility = input.eligibility();
        boolean eligible = PeaceReference.canUseAction(eligibility.enabled(),
                eligibility.concernCanMakePeace(), eligibility.targetHostile() != null,
                Boolean.TRUE.equals(eligibility.targetHostile()), eligibility.factionDisabled());
        // MakePeaceAction.applyPriorityModifiers, a669f4d: only its own MULTIPLIER write.
        // MIT copyright 2015 L.J. "Histidine" Lim; inherited MutableStat totals are unavailable.
        float multiplier = Math.min(input.concern().weariness() / input.concern().minimumForPeace(), 5);
        PriorityEntry priority = PriorityEntry.of("weariness", PriorityEntry.Kind.MULTIPLIER, multiplier);
        return new Output(concern, WarWearinessReference.isValid(input.concern()), eligible,
                priority, PeaceReference.evaluateSelectedEnemy(input.selected()));
    }
}
