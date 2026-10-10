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
import java.util.Collections;
import java.util.List;

/** Copied-fact untargeted traversal after checkPeace's outer guards; performs no game calls. */
public final class PeaceTraversalReference {
    private PeaceTraversalReference() {}

    public enum ReportedReturn { NULL, NON_NULL, NOT_SUPPLIED }
    public enum Step {
        RECENT_WAR, CANNOT_CEASEFIRE, COMMISSIONED_PLAYER, OFFENSIVE_BLOCKED,
        NULL_RETURN, NON_NULL_RETURN
    }
    public record Enemy(String id, int rawWearinessBits, boolean recentWar, boolean canCeasefire,
            boolean commissionedPlayer, boolean offensiveBlocked, ReportedReturn reportedReturn) {
        public Enemy {
            if (id == null || id.isEmpty() || id.length() > 128 || reportedReturn == null) {
                throw new IllegalArgumentException("Missing copied traversal fact");
            }
        }
    }
    public record Visit(String enemy, Step step) {}
    public record CacheRefreshRequest(int deltaFloatBits) {}
    public record Result(List<String> orderedEnemies, List<Visit> visits, int nullAttempts,
            String returnedEnemy, List<CacheRefreshRequest> cacheRefreshRequests) {
        public Result {
            orderedEnemies = List.copyOf(orderedEnemies);
            visits = List.copyOf(visits);
            cacheRefreshRequests = List.copyOf(cacheRefreshRequests);
        }
    }

    // DiplomacyBrain.checkPeace lines622-669, source a669f4d0740e95a4acbb6b894dde09ade67aa754.
    // Outer gates are a prerequisite, targetFactionId is null, getters/returns are supplied facts.
    // Actual comparator is ascending despite its source comment. No float rejection is invented.
    public static Result evaluatePassedOuterGates(List<Enemy> supplied) {
        ReferenceBounds.count(supplied, 256);
        List<Enemy> ordered = new ArrayList<>(List.copyOf(supplied));
        Collections.sort(ordered, (first, second) -> Float.compare(
                Float.intBitsToFloat(first.rawWearinessBits()),
                Float.intBitsToFloat(second.rawWearinessBits())));
        List<String> ids = ordered.stream().map(Enemy::id).toList();
        List<Visit> visits = new ArrayList<>();
        int nullAttempts = 0;
        for (Enemy enemy : ordered) {
            Step skip = skip(enemy);
            if (skip != null) {
                visits.add(new Visit(enemy.id(), skip));
                continue;
            }
            if (enemy.reportedReturn() == ReportedReturn.NOT_SUPPLIED) {
                throw new IllegalArgumentException("Missing reported tryMakePeace return");
            }
            if (enemy.reportedReturn() == ReportedReturn.NON_NULL) {
                visits.add(new Visit(enemy.id(), Step.NON_NULL_RETURN));
                return new Result(ids, visits, nullAttempts, enemy.id(),
                        List.of(new CacheRefreshRequest(0x00000000)));
            }
            visits.add(new Visit(enemy.id(), Step.NULL_RETURN));
            nullAttempts++;
            if (nullAttempts >= 3) break;
        }
        return new Result(ids, visits, nullAttempts, null, List.of());
    }
    // DiplomacyBrain.checkPeace lines622-639: a supplied target is a singleton, even outside enemies.
    // All outer gates remain prerequisites; no fallback or game lookup is performed here.
    public static Result evaluateSelectionPassedOuterGates(List<Enemy> supplied, Enemy suppliedTarget) {
        return evaluatePassedOuterGates(suppliedTarget == null ? supplied : List.of(suppliedTarget));
    }
    private static Step skip(Enemy enemy) {
        if (enemy.recentWar()) return Step.RECENT_WAR;
        if (!enemy.canCeasefire()) return Step.CANNOT_CEASEFIRE;
        if (enemy.commissionedPlayer()) return Step.COMMISSIONED_PLAYER;
        if (enemy.offensiveBlocked()) return Step.OFFENSIVE_BLOCKED;
        return null;
    }
}
