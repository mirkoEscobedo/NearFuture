package nf.nex.reference;

import java.util.List;
import java.util.Set;

public record PriorityInputs(float diplomaticAlignment, Set<String> traits, Float factionMultiplier,
        float maximumAlignmentModifier, float positiveTraitMultiplier, float negativeTraitMultiplier,
        List<String> orderedTags, List<PriorityEntry> existingPriority) {
    public PriorityInputs {
        ReferenceBounds.finite(diplomaticAlignment, maximumAlignmentModifier, positiveTraitMultiplier, negativeTraitMultiplier);
        if (factionMultiplier != null) ReferenceBounds.finite(factionMultiplier);
        ReferenceBounds.count(traits, 64);
        ReferenceBounds.count(orderedTags, 64);
        ReferenceBounds.count(existingPriority, 64);
        Set<String> expectedTags = Set.of("diplomacy", "canMakePeace", "trait_pacifist", "trait_weak-willed", "!trait_stalwart");
        if (orderedTags.size() != 5 || !Set.copyOf(orderedTags).equals(expectedTags)
                || !Set.of("pacifist", "weak-willed", "stalwart").containsAll(traits)) {
            throw new IllegalArgumentException("Uncharacterized trait/tag semantics");
        }
        traits = Set.copyOf(traits);
        orderedTags = List.copyOf(orderedTags);
        existingPriority = List.copyOf(existingPriority);
    }
}
