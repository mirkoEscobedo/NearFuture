package nf.nex.reference;

import java.util.List;
import java.util.Set;

public final class ReferenceSmoke {
    private ReferenceSmoke() {}
    private static WarInput input(float weariness) {
        PriorityInputs priority = new PriorityInputs(0.5f, Set.of(), null, 0.25f, 1.3f, 0.7f,
                List.of("diplomacy", "canMakePeace", "trait_pacifist", "trait_weak-willed", "!trait_stalwart"), List.of());
        return new WarInput(weariness, 5000f, false, true, List.of(), priority);
    }
    public static void main(String[] args) throws Exception {
        WarResult result = WarWearinessReference.generate(input(Math.nextDown(3750f)));
        if (result.generated() || !result.ended() || !result.abortCurrentAction() || !result.writes().isEmpty()) {
            throw new AssertionError("Below-threshold reference must end and abort without priority writes");
        }
        WarInput original = input(5000f);
        WarInput existing = new WarInput(original.weariness(), original.minimumForPeace(), false, false,
                List.of(new WarInput.ExistingConcern(WarInput.ConcernClass.WAR_WEARINESS, false)), original.priority());
        WarResult suppressed = WarWearinessReference.generate(existing);
        if (suppressed.generated() || suppressed.ended() || !suppressed.writes().isEmpty()) {
            throw new AssertionError("Active exact-class concern must suppress generation before update");
        }
        boolean rejected = false;
        try { WarWearinessReference.isValid(input(Float.NaN)); }
        catch (IllegalArgumentException expected) { rejected = true; }
        if (!rejected) throw new AssertionError("Nonfinite reference input must be unavailable");
        ReferenceLifecycle.run();
        System.out.println("PASS: copied-fact peace traversal checks " + PeaceTraversalCharacterization.run());
        System.out.println("PASS: isolated concern/peace source corpus " + WarCorpus.run() + "/" + PeaceCorpus.run());
    }
}
