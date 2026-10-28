package nf.adapter.ipc;

import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.Map;

/** Closed startup fault options are validated before any worker or process exists. */
public final class DiagnosticFaultConfigurationTest {
    private static final String PREFIX = "nf.ipc.diagnostic.";
    private DiagnosticFaultConfigurationTest() { }

    public static void main(String[] args) {
        Map<String, String> settings = validSettings();
        Map<String, String> previous = new LinkedHashMap<>();
        try {
            settings.forEach((name, value) -> {
                previous.put(name, System.getProperty(name));
                System.setProperty(name, value);
            });
            if (DiagnosticConfiguration.configured() == null) {
                throw new AssertionError("valid enabled diagnostic configuration was unavailable");
            }
            // Independent literal oracle: the diagnostic delay cap is 5000ms.
            System.setProperty(PREFIX + "handoffDelayMillis", "5001");
            try {
                DiagnosticConfiguration.configured();
                throw new AssertionError("out-of-range diagnostic handoff delay5001 was admitted");
            } catch (NodeFailure refused) {
                if (refused.code() != NodeFailure.Code.INVALID) {
                    throw new AssertionError("out-of-range handoff delay must be INVALID");
                }
            }
        } finally {
            previous.forEach((name, value) -> {
                if (value == null) System.clearProperty(name);
                else System.setProperty(name, value);
            });
        }
        System.out.println("Closed diagnostic handoff delay configuration refusal passed.");
    }

    private static Map<String, String> validSettings() {
        Path root = Path.of(".").toAbsolutePath().normalize();
        Map<String, String> values = new LinkedHashMap<>();
        values.put(PREFIX + "enabled", "true");
        // These paths are values only. This test creates no files or processes.
        values.put(PREFIX + "executable", root.resolve("not-executed-test-node").toString());
        values.put(PREFIX + "sha256", "11".repeat(32));
        values.put(PREFIX + "vault", root.resolve("not-created-test-vault").toString());
        values.put(PREFIX + "saves", root.resolve("not-created-test-saves").toString());
        values.put(PREFIX + "publication", "fault-configuration-test");
        values.put(PREFIX + "universe", "81".repeat(16));
        values.put(PREFIX + "history", "82".repeat(16));
        values.put(PREFIX + "ruleset", "83".repeat(32));
        values.put(PREFIX + "content", "84".repeat(32));
        values.put(PREFIX + "account", "85".repeat(16));
        values.put(PREFIX + "device", "86".repeat(16));
        values.put(PREFIX + "handoffDelayMillis", "0");
        values.put(PREFIX + "retainStalePublication", "false");
        return values;
    }
}
