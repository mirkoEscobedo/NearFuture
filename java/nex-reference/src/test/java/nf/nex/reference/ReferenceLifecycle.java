package nf.nex.reference;

import java.util.List;
import java.util.Set;

final class ReferenceLifecycle {
    private ReferenceLifecycle() {}
    private static PriorityInputs priority() {
        return new PriorityInputs(0.5f, Set.of(), null, 0.25f, 1.3f, 0.7f,
                List.of("diplomacy", "canMakePeace", "trait_pacifist", "trait_weak-willed", "!trait_stalwart"), List.of());
    }
    static void run() {
        WarInput first = new WarInput(5000f, 5000f, false, true, List.of(), priority());
        WarResult active = WarWearinessReference.update(first);
        if (active.ended() || active.abortCurrentAction() || active.writes().size() != 2) throw new AssertionError("Initial update");
        WarInput below = new WarInput(3000f, 5000f, false, true, List.of(), priority());
        WarResult ended = WarWearinessReference.update(below);
        if (!ended.ended() || !ended.abortCurrentAction() || !ended.writes().isEmpty()) throw new AssertionError("Termination");
        WarInput staleObject = new WarInput(5000f, 5000f, ended.ended(), false, List.of(), priority());
        if (WarWearinessReference.generate(staleObject).generated()) throw new AssertionError("Ended object resurrected");
        WarInput replacement = new WarInput(5000f, 5000f, false, false,
                List.of(new WarInput.ExistingConcern(WarInput.ConcernClass.WAR_WEARINESS, true)), priority());
        if (!WarWearinessReference.generate(replacement).generated()) throw new AssertionError("Replacement not restarted");
        if (!PeaceReference.canUseAction(true, true, false, false, false)
                || PeaceReference.canUseAction(false, true, false, false, false)
                || PeaceReference.canUseAction(true, false, false, false, false)
                || PeaceReference.canUseAction(true, true, true, false, false)
                || PeaceReference.canUseAction(true, true, false, true, true)) {
            throw new AssertionError("MakePeaceAction exact override eligibility");
        }
        PeaceInput source = PeaceCorpus.input("none", true, List.of(new SuppliedDraw("peace-chance", 0x3fe0000000000000L)));
        expectUnavailable(() -> PeaceReference.evaluateSelectedEnemy(source));
        PeaceInput player = new PeaceInput(source.faction(), source.enemy(), true, false, source.ownWeariness(), source.enemyWeariness(),
                source.ownEventDisposition(), source.enemyEventDisposition(), source.gates(), source.rules(), source.draws());
        expectUnavailable(() -> PeaceReference.evaluateSelectedEnemy(player));
        PeaceGates absent = new PeaceGates(true, false, false, 30f, 30f, false, true, false, false, false);
        PeaceInput missing = new PeaceInput(source.faction(), source.enemy(), false, false, source.ownWeariness(), source.enemyWeariness(),
                source.ownEventDisposition(), source.enemyEventDisposition(), absent, source.rules(), source.draws());
        expectUnavailable(() -> PeaceReference.evaluateSelectedEnemy(missing));
        expectUnavailable(() -> new SuppliedDraw("peace-chance", 0x7ff0000000000000L));
        expectUnavailable(() -> new SuppliedDraw("peace-chance", 0x3ff0000000000000L));
    }
    private static void expectUnavailable(Runnable action) {
        try { action.run(); } catch (IllegalArgumentException expected) { return; }
        throw new AssertionError("Uncharacterized reference input accepted");
    }
}
