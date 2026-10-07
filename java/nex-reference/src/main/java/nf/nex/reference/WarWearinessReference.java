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

import java.util.ArrayList;
import java.util.List;

/** Adapted v0.12.2c WarWearinessConcern and required priority expressions; effect traces only. */
public final class WarWearinessReference {
    private WarWearinessReference() {}

    // WarWearinessConcern.generate/update/isValid, commit a669f4d0740e95a4acbb6b894dde09ade67aa754.
    public static WarResult generate(WarInput input) {
        for (WarInput.ExistingConcern existing : input.existingConcerns()) {
            if (!existing.ended() && existing.exactClass() == WarInput.ConcernClass.WAR_WEARINESS) {
                return new WarResult(false, input.alreadyEnded(), false, input.priority().existingPriority(), List.of());
            }
        }
        WarResult update = update(input);
        return new WarResult(!update.ended(), update.ended(), update.abortCurrentAction(),
                update.existingPriority(), update.writes());
    }

    public static WarResult update(WarInput input) {
        float weariness = input.weariness();
        if (weariness < input.minimumForPeace() * 0.75f) {
            return new WarResult(false, true, input.currentActionPresent(), input.priority().existingPriority(), List.of());
        }
        return new WarResult(false, input.alreadyEnded(), false, input.priority().existingPriority(), priorityWrites(input));
    }

    public static boolean isValid(WarInput input) {
        return input.weariness() >= input.minimumForPeace() * 0.75f;
    }

    // BaseStrategicConcern.reapplyPriorityModifiers and SAIUtils alignment/trait expressions.
    private static List<PriorityEntry> priorityWrites(WarInput input) {
        List<PriorityEntry> writes = new ArrayList<>();
        writes.add(PriorityEntry.of("value", PriorityEntry.Kind.FLAT, input.weariness() / 100f));
        PriorityInputs p = input.priority();
        if (p.orderedTags().contains("diplomacy")) {
            writes.add(PriorityEntry.of("alignment_diplomatic", PriorityEntry.Kind.MULTIPLIER,
                    1 + p.maximumAlignmentModifier() * p.diplomaticAlignment()));
        }
        for (String tag : p.orderedTags()) {
            if (tag.startsWith("trait_")) {
                String traitId = tag.substring("trait_".length());
                if (p.traits().contains(traitId)) {
                    writes.add(PriorityEntry.of("trait_" + traitId, PriorityEntry.Kind.MULTIPLIER, p.positiveTraitMultiplier()));
                }
            } else if (tag.startsWith("!trait_")) {
                String traitId = tag.substring("!trait_".length());
                if (p.traits().contains(traitId)) {
                    writes.add(PriorityEntry.of("trait_" + traitId, PriorityEntry.Kind.MULTIPLIER, p.negativeTraitMultiplier()));
                }
            }
        }
        if (p.factionMultiplier() != null) {
            writes.add(PriorityEntry.of("faction", PriorityEntry.Kind.MULTIPLIER, p.factionMultiplier()));
        }
        return writes;
    }
}
