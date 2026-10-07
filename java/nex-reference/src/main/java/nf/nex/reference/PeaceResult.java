package nf.nex.reference;

import java.util.List;

public record PeaceResult(Decision decision, int consumedDraws, List<EffectCall> effects) {
    public enum Decision { NONE, CEASEFIRE, TREATY }
    public sealed interface EffectCall permits DiplomacyEventCall, WearinessCall {}
    public record DiplomacyEventCall(String faction, String enemy, String eventId) implements EffectCall {}
    public record WearinessCall(String faction, int amountFloatBits) implements EffectCall {}
    public PeaceResult { effects = List.copyOf(effects); }
}
