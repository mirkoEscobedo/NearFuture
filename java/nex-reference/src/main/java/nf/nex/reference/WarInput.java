package nf.nex.reference;

import java.util.List;

public record WarInput(float weariness, float minimumForPeace, boolean alreadyEnded, boolean currentActionPresent,
        List<ExistingConcern> existingConcerns, PriorityInputs priority) {
    public enum ConcernClass { WAR_WEARINESS, OTHER }
    public record ExistingConcern(ConcernClass exactClass, boolean ended) {
        public ExistingConcern { if (exactClass == null) throw new IllegalArgumentException("Missing exact concern class"); }
    }
    public WarInput {
        ReferenceBounds.finite(weariness, minimumForPeace);
        ReferenceBounds.count(existingConcerns, 256);
        if (priority == null) throw new IllegalArgumentException("Missing priority facts");
        existingConcerns = List.copyOf(existingConcerns);
    }
}
