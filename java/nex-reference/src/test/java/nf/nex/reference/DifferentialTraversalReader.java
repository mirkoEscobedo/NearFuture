package nf.nex.reference;

import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.stream.Collectors;
import static nf.nex.reference.PeaceTraversalReference.*;

/** Only public synthetic raw columns; each mode calls its own copied-fact evaluator. */
final class DifferentialTraversalReader {
    private DifferentialTraversalReader() {}
    static String output(String[] fields) {
        if (fields.length != 2) throw new IllegalArgumentException("Invalid traversal shape");
        List<Enemy> input = input(fields[1]); // Parse refusal remains an oracle-process failure.
        final Result result;
        try { result = evaluatePassedOuterGates(input); }
        catch (IllegalArgumentException unavailable) { return "UNAVAILABLE"; }
        return output(result);
    }
    static String outputSelection(String[] fields) {
        if (fields.length != 3) throw new IllegalArgumentException("Invalid targeted traversal shape");
        List<Enemy> pool = input(fields[1]);
        final Enemy target;
        if (fields[2].equals("-")) target = null;
        else {
            List<Enemy> targets = input(fields[2]);
            if (targets.size() != 1) throw new IllegalArgumentException("Expected one target fact");
            target = targets.get(0);
        }
        final Result result;
        try { result = evaluateSelectionPassedOuterGates(pool, target); }
        catch (IllegalArgumentException unavailable) { return "UNAVAILABLE"; }
        return output(result);
    }
    private static String output(Result result) {
        String ordered = result.orderedEnemies().stream().collect(Collectors.joining(","));
        String visits = result.visits().stream().map(v -> v.enemy() + ":" + v.step())
                .collect(Collectors.joining(","));
        String refresh = result.cacheRefreshRequests().stream()
                .map(r -> String.format(Locale.ROOT, "%08x", r.deltaFloatBits()))
                .collect(Collectors.joining(","));
        return "OK|" + empty(ordered) + "|" + empty(visits) + "|" + result.nullAttempts()
                + "|" + (result.returnedEnemy() == null ? "-" : result.returnedEnemy()) + "|" + empty(refresh);
    }
    private static List<Enemy> input(String text) {
        if (text.equals("-")) return List.of();
        var result = new ArrayList<Enemy>();
        for (String entry : text.split(";", -1)) {
            String[] f = entry.split(",", -1);
            if (f.length != 7 || !f[0].matches("[a-z_]{1,32}") || !f[1].matches("[0-9a-f]{8}")) {
                throw new IllegalArgumentException("Invalid public copied fact");
            }
            ReportedReturn returned = switch (f[6]) {
                case "N" -> ReportedReturn.NULL; case "R" -> ReportedReturn.NON_NULL;
                case "U" -> ReportedReturn.NOT_SUPPLIED;
                default -> throw new IllegalArgumentException("Invalid reported return");
            };
            result.add(new Enemy(f[0], Integer.parseUnsignedInt(f[1], 16),
                    DifferentialWarReader.bool(f[2]), DifferentialWarReader.bool(f[3]),
                    DifferentialWarReader.bool(f[4]), DifferentialWarReader.bool(f[5]), returned));
        }
        return List.copyOf(result);
    }
    private static String empty(String value) { return value.isEmpty() ? "-" : value; }
}
