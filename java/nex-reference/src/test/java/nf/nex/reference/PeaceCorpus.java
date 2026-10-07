package nf.nex.reference;

import java.io.BufferedReader;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

final class PeaceCorpus {
    private PeaceCorpus() {}
    static PeaceInput input(String gate, boolean treatyRelation, List<SuppliedDraw> draws) {
        PeaceGates gates = new PeaceGates(!gate.equals("noEnemies"), gate.equals("pirate"), false,
                gate.equals("warInterval") ? Float.intBitsToFloat(0x41efffff) : 30f, 30f,
                gate.equals("recentWar"), !gate.equals("canCeasefire"), gate.equals("commissioned"), gate.equals("offensive"), true);
        PeaceRules rules = new PeaceRules(5000f, 20000f, 75f, 0, 40f, 0.3f, 3000f, 6000f);
        float below = Float.intBitsToFloat(0x459c3fff);
        return new PeaceInput("hegemony", "tritachyon", false, treatyRelation,
                gate.equals("ownWeariness") ? below : 5000f, gate.equals("enemyWeariness") ? below : 5000f,
                0, 0, gates, rules, draws);
    }
    static int run() throws Exception {
        InputStream stream = PeaceCorpus.class.getResourceAsStream("/peace-v1.tsv");
        if (stream == null) throw new AssertionError("Missing peace corpus");
        int count = 0;
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(stream, StandardCharsets.UTF_8))) {
            for (String row; (row = reader.readLine()) != null;) {
                if (row.startsWith("#") || row.isBlank()) continue;
                String[] fields = row.split("\\|", -1);
                if (fields.length != 8) throw new AssertionError("Bad peace row");
                List<SuppliedDraw> draws = new ArrayList<>();
                draws.add(new SuppliedDraw("peace-chance", Long.parseUnsignedLong(fields[3], 16)));
                if (!fields[4].equals("-")) draws.add(new SuppliedDraw("peace-treaty", Long.parseUnsignedLong(fields[4], 16)));
                PeaceInput input = input(fields[1], Boolean.parseBoolean(fields[2]), draws);
                PeaceResult result = PeaceReference.evaluateSelectedEnemy(input);
                if (result.decision() != PeaceResult.Decision.valueOf(fields[5]) || result.consumedDraws() != Integer.parseInt(fields[6])) {
                    throw new AssertionError(fields[0] + " lifecycle/draws differ");
                }
                List<PeaceResult.EffectCall> expected = List.of();
                if (!fields[7].equals("-")) {
                    int amount = Integer.parseUnsignedInt(fields[7], 16);
                    expected = List.of(new PeaceResult.DiplomacyEventCall("hegemony", "tritachyon",
                            fields[5].equals("TREATY") ? "peace_treaty" : "ceasefire"),
                            new PeaceResult.WearinessCall("hegemony", amount), new PeaceResult.WearinessCall("tritachyon", amount));
                }
                if (!result.effects().equals(expected)) throw new AssertionError(fields[0] + " effects differ");
                count++;
            }
        }
        if (count != 16) throw new AssertionError("Incomplete peace corpus");
        return count;
    }
}
