/*
 * The MIT License (MIT)
 *
 * Copyright (c) 2015 L.J. "Histidine" Lim
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */
package nf.nex.reference;

import java.util.List;

/** Adapted selected non-player checkPeace/tryMakePeace source; returns requested calls, performs none. */
public final class PeaceReference {
    private PeaceReference() {}

    // DiplomacyBrain.checkPeace + tryMakePeace, commit a669f4d0740e95a4acbb6b894dde09ade67aa754.
    public static PeaceResult evaluateSelectedEnemy(PeaceInput input) {
        if (input.enemyIsPlayer() || !input.gates().offensiveFactsProvided()) {
            throw new IllegalArgumentException("Player/offensive coverage unavailable");
        }
        PeaceGates gates = input.gates();
        PeaceRules rules = input.rules();
        if (!gates.hasEnemies() || (gates.pirateFaction() && !gates.allowPirateInvasions())
                || gates.daysSinceLastWar() < gates.minimumWarInterval()
                || input.ownWeariness() < rules.minimumWeariness()
                || gates.recentWar() || !gates.canCeasefire() || gates.commissionedTarget() || gates.offensiveBlocked()
                || input.enemyWeariness() < rules.minimumWeariness()) {
            return none(0);
        }
        float sumWeariness = input.ownWeariness() + input.enemyWeariness();
        float eventsMod = input.ownEventDisposition() + input.enemyEventDisposition();
        eventsMod *= rules.eventPeaceMultiplier();
        sumWeariness += eventsMod;
        float divisor = rules.divisor() + rules.divisorPerLevel() * rules.playerLevel();
        ReferenceBounds.finite(sumWeariness, divisor);
        if (divisor <= 0) throw new IllegalArgumentException("Unsupported peace divisor");
        float chance = sumWeariness / divisor;
        ReferenceBounds.finite(chance);
        if (draw(input, 0, "peace-chance") > chance) return none(1);
        boolean peaceTreaty = false;
        int consumed = 1;
        if (input.treatyRelationEligible()) {
            peaceTreaty = draw(input, 1, "peace-treaty") < rules.treatyChance();
            consumed++;
        }
        String eventId = peaceTreaty ? "peace_treaty" : "ceasefire";
        float reduction = peaceTreaty ? rules.treatyReduction() : rules.ceasefireReduction();
        return new PeaceResult(peaceTreaty ? PeaceResult.Decision.TREATY : PeaceResult.Decision.CEASEFIRE,
                consumed, List.of(
                        new PeaceResult.DiplomacyEventCall(input.faction(), input.enemy(), eventId),
                        new PeaceResult.WearinessCall(input.faction(), Float.floatToRawIntBits(-reduction)),
                        new PeaceResult.WearinessCall(input.enemy(), Float.floatToRawIntBits(-reduction))));
    }

    // MakePeaceAction.canUse overrides its base; no inherited global-cooldown guard is invented.
    public static boolean canUseAction(boolean enableDiplomacy, boolean concernCanMakePeace,
            boolean hasTarget, boolean targetHostile, boolean factionDiplomacyDisabled) {
        if (!enableDiplomacy) return false;
        if (!concernCanMakePeace) return false;
        if (hasTarget && !targetHostile) return false;
        return !factionDiplomacyDisabled;
    }

    private static double draw(PeaceInput input, int ordinal, String purpose) {
        if (input.draws().size() <= ordinal || !input.draws().get(ordinal).purpose().equals(purpose)) {
            throw new IllegalArgumentException("Missing or out-of-order reference draw");
        }
        return input.draws().get(ordinal).value();
    }
    private static PeaceResult none(int consumed) { return new PeaceResult(PeaceResult.Decision.NONE, consumed, List.of()); }
}
