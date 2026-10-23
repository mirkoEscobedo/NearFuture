package nf.nex.reference;

import java.util.Locale;
import java.util.stream.Collectors;

/** All 24 columns are identifiers, selectors or raw supplied facts; no expected output columns. */
final class DifferentialFirstProposalReader {
    private DifferentialFirstProposalReader() { }
    static String output(String[] f) {
        if (f.length != 24) throw new IllegalArgumentException("Invalid copied-facet shape");
        WarInput concern = DifferentialWarReader.input(new String[] {
                f[0], f[2], f[3], f[4], f[5], f[6], f[7], f[8], "-", "-", "-", "-", "-" });
        FirstProposalReference.Eligibility eligibility = new FirstProposalReference.Eligibility(
                bool(f[9]), bool(f[10]), f[11].equals("-") ? null : bool(f[11]), bool(f[12]));
        PeaceInput defaults = DifferentialPeaceReader.input(new String[] {
                f[0], f[13], f[14], f[15], f[16], "-", "-", "-" });
        PeaceRules r = defaults.rules();
        PeaceRules rules = new PeaceRules(value(f[23]), r.divisor(), r.divisorPerLevel(),
                r.playerLevel(), r.eventPeaceMultiplier(), r.treatyChance(),
                r.ceasefireReduction(), r.treatyReduction());
        PeaceInput selected = new PeaceInput(defaults.faction(), f[17], bool(f[21]),
                defaults.treatyRelationEligible(), value(f[22]), value(f[18]),
                value(f[19]), value(f[20]), defaults.gates(), rules, defaults.draws());
        FirstProposalReference.Output result = FirstProposalReference.evaluate(new FirstProposalReference.Input(
                FirstProposalReference.Operation.valueOf(f[1]), concern, eligibility, selected));
        WarResult war = result.concern();
        PeaceResult peace = result.selected();
        String effects = peace.effects().stream().map(effect ->
                effect instanceof PeaceResult.DiplomacyEventCall e
                        ? "event:" + e.faction() + ":" + e.enemy() + ":" + e.eventId()
                        : effect instanceof PeaceResult.WearinessCall w
                        ? "weariness:" + w.faction() + ":" + String.format(Locale.ROOT, "%08x", w.amountFloatBits())
                        : "unsupported").collect(Collectors.joining(","));
        return war.generated() + "|" + war.ended() + "|" + war.abortCurrentAction() + "|" + result.valid()
                + "|" + priority(war.existingPriority()) + "|" + priority(war.writes())
                + "|" + result.eligible() + "|" + priority(java.util.List.of(result.additionalPriority()))
                + "|" + peace.decision() + "|" + peace.consumedDraws() + "|" + (effects.isEmpty() ? "-" : effects);
    }
    private static boolean bool(String text) { return DifferentialWarReader.bool(text); }
    private static float value(String text) { return DifferentialWarReader.value(text); }
    private static String priority(java.util.List<PriorityEntry> entries) {
        String text = entries.stream().map(entry -> entry.id() + ":" + entry.kind() + ":"
                + String.format(Locale.ROOT, "%08x", entry.floatBits())).collect(Collectors.joining(","));
        return text.isEmpty() ? "-" : text;
    }
}
