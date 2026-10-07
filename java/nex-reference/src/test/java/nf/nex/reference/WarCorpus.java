package nf.nex.reference;

import java.io.BufferedReader;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import java.util.stream.Collectors;

final class WarCorpus {
    private WarCorpus() {}
    static int run() throws Exception {
        InputStream stream = WarCorpus.class.getResourceAsStream("/war-v1.tsv");
        if (stream == null) throw new AssertionError("Missing independent reference corpus");
        int count = 0;
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(stream, StandardCharsets.UTF_8))) {
            for (String row; (row = reader.readLine()) != null;) {
                if (row.startsWith("#") || row.isBlank()) continue;
                String[] fields = row.split("\\|", -1);
                if (fields.length != 13) throw new AssertionError("Bad corpus row");
                Set<String> traits = new HashSet<>();
                int mask = Integer.parseInt(fields[6]);
                if ((mask & 1) != 0) traits.add("pacifist");
                if ((mask & 2) != 0) traits.add("weak-willed");
                if ((mask & 4) != 0) traits.add("stalwart");
                List<WarInput.ExistingConcern> existing = new ArrayList<>();
                switch (fields[5]) {
                    case "none" -> { }
                    case "active" -> existing.add(new WarInput.ExistingConcern(WarInput.ConcernClass.WAR_WEARINESS, false));
                    case "ended" -> existing.add(new WarInput.ExistingConcern(WarInput.ConcernClass.WAR_WEARINESS, true));
                    case "other" -> existing.add(new WarInput.ExistingConcern(WarInput.ConcernClass.OTHER, false));
                    default -> throw new AssertionError("Unknown fixture concern class");
                }
                Float multiplier = fields[7].equals("-") ? null : value(fields[7]);
                List<PriorityEntry> prior = List.of(new PriorityEntry("value", PriorityEntry.Kind.FLAT, 0x3f800000));
                PriorityInputs priority = new PriorityInputs(0.5f, traits, multiplier, 0.25f, 1.3f, 0.7f,
                        List.of("diplomacy", "canMakePeace", "trait_pacifist", "trait_weak-willed", "!trait_stalwart"), prior);
                WarInput input = new WarInput(value(fields[1]), value(fields[2]), Boolean.parseBoolean(fields[3]),
                        Boolean.parseBoolean(fields[4]), existing, priority);
                WarResult result = WarWearinessReference.generate(input);
                String writes = result.writes().isEmpty() ? "-" : result.writes().stream().map(entry -> entry.id() + ":" + entry.kind()
                        + ":" + String.format(Locale.ROOT, "%08x", entry.floatBits())).collect(Collectors.joining(","));
                if (result.generated() != Boolean.parseBoolean(fields[8]) || result.ended() != Boolean.parseBoolean(fields[9])
                        || result.abortCurrentAction() != Boolean.parseBoolean(fields[10])
                        || WarWearinessReference.isValid(input) != Boolean.parseBoolean(fields[11])
                        || !writes.equals(fields[12]) || !result.existingPriority().equals(prior)) {
                    throw new AssertionError(fields[0] + " differs: " + writes);
                }
                count++;
            }
        }
        if (count != 10) throw new AssertionError("Incomplete corpus");
        return count;
    }
    private static float value(String bits) { return Float.intBitsToFloat(Integer.parseUnsignedInt(bits, 16)); }
}
