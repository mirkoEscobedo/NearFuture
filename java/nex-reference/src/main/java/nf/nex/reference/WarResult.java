package nf.nex.reference;

import java.util.List;

public record WarResult(boolean generated, boolean ended, boolean abortCurrentAction,
        List<PriorityEntry> existingPriority, List<PriorityEntry> writes) {
    public WarResult { existingPriority = List.copyOf(existingPriority); writes = List.copyOf(writes); }
}
