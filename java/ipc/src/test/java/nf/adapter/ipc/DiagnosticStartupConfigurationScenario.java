package nf.adapter.ipc;

import java.util.ArrayList;
import java.util.HexFormat;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** Startup configuration is consumed before activation; all private property values are restored. */
final class DiagnosticStartupConfigurationScenario {
    private static final String PREFIX = "nf.ipc.diagnostic.";
    private DiagnosticStartupConfigurationScenario() { }

    static void run(NodeLaunch launch) throws InterruptedException {
        Map<String, String> settings = settings(launch);
        Map<String, String> previous = new LinkedHashMap<>();
        settings.keySet().forEach(name -> previous.put(name, System.getProperty(name)));
        try {
            List<String> defects = closedControls(settings);
            settings.forEach(System::setProperty);
            try {
                DiagnosticStaleMailboxScenario.run(launch, () -> {
                    try { return DiagnosticConfiguration.configuredOwner(); }
                    finally { restore(previous); }
                }, 750);
            } catch (AssertionError realFailure) {
                // Retain the actual process/JFR failure; never replace setup failures with a RED label.
                if (!defects.isEmpty()) realFailure.addSuppressed(new AssertionError(String.join("; ", defects)));
                throw realFailure;
            }
            if (!defects.isEmpty()) throw new AssertionError(String.join("; ", defects));
        } finally { restore(previous); }
    }

    private static List<String> closedControls(Map<String, String> settings) {
        List<String> defects = new ArrayList<>();
        settings.forEach(System::setProperty);
        System.clearProperty(PREFIX + "enabled");
        System.setProperty(PREFIX + "handoffDelayMillis", "5001");
        System.setProperty(PREFIX + "retainStalePublication", "TRUE");
        if (DiagnosticConfiguration.configured() != null || DiagnosticConfiguration.configuredOwner() != null) {
            defects.add("disabled startup configuration admitted an owner");
        }
        settings.forEach(System::setProperty);
        System.clearProperty(PREFIX + "handoffDelayMillis");
        System.clearProperty(PREFIX + "retainStalePublication");
        if (DiagnosticConfiguration.configured() == null) defects.add("missing optional fault options refused enabled launch");
        for (String delay : new String[]{"0", "5000"}) {
            for (String retain : new String[]{"true", "false"}) {
                settings.forEach(System::setProperty);
                System.setProperty(PREFIX + "handoffDelayMillis", delay);
                System.setProperty(PREFIX + "retainStalePublication", retain);
                if (DiagnosticConfiguration.configured() == null) defects.add("closed valid startup option refused");
            }
        }
        for (String delay : new String[]{"-1", "5001", " 1", "1 ", "01", "1.0", "", "99999"}) {
            settings.forEach(System::setProperty);
            System.setProperty(PREFIX + "handoffDelayMillis", delay);
            refused("delay", defects);
        }
        for (String retain : new String[]{"TRUE", "FALSE", "1", " true", "false ", "", "yes"}) {
            settings.forEach(System::setProperty);
            System.setProperty(PREFIX + "retainStalePublication", retain);
            refused("retain literal", defects);
        }
        return defects;
    }

    private static void refused(String option, List<String> defects) {
        try {
            DiagnosticConfiguration.configured();
            defects.add("invalid " + option + " admitted");
        } catch (NodeFailure refused) {
            if (refused.code() != NodeFailure.Code.INVALID) defects.add("invalid " + option + " wrong refusal");
        }
    }

    static Map<String, String> settings(NodeLaunch launch) {
        Map<String, String> values = new LinkedHashMap<>();
        values.put(PREFIX + "enabled", "true");
        values.put(PREFIX + "executable", launch.executable().toString());
        values.put(PREFIX + "sha256", hex(launch.executableSha256().toByteArray()));
        values.put(PREFIX + "vault", launch.vault().toString());
        values.put(PREFIX + "saves", launch.saves().toString());
        values.put(PREFIX + "publication", launch.publication());
        var scope = launch.scope();
        values.put(PREFIX + "universe", hex(scope.universe().toByteArray()));
        values.put(PREFIX + "history", hex(scope.history().toByteArray()));
        values.put(PREFIX + "ruleset", hex(scope.ruleset().toByteArray()));
        values.put(PREFIX + "content", hex(scope.content().toByteArray()));
        values.put(PREFIX + "account", hex(scope.principal().getAccountId().getValue().toByteArray()));
        values.put(PREFIX + "device", hex(scope.principal().getDeviceId().getValue().toByteArray()));
        values.put(PREFIX + "handoffDelayMillis", "750");
        values.put(PREFIX + "retainStalePublication", "true");
        return values;
    }

    private static String hex(byte[] value) { return HexFormat.of().formatHex(value); }
    static void restore(Map<String, String> previous) {
        previous.forEach((name, value) -> {
            if (value == null) System.clearProperty(name);
            else System.setProperty(name, value);
        });
    }
}
