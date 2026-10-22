package nf.adapter.ipc;

import java.util.LinkedHashMap;
import java.util.Map;

/** Both absent options and explicit0/false must remain inert through real owner lifecycles. */
final class DiagnosticDefaultOptionsScenario {
    private static final String PREFIX = "nf.ipc.diagnostic.";
    private DiagnosticDefaultOptionsScenario() { }

    static void run(NodeLaunch launch, boolean explicit) throws InterruptedException {
        Map<String, String> settings = DiagnosticStartupConfigurationScenario.settings(launch);
        Map<String, String> previous = new LinkedHashMap<>();
        settings.keySet().forEach(name -> previous.put(name, System.getProperty(name)));
        try {
            DiagnosticStaleMailboxScenario.run(launch, () -> {
                try {
                    settings.forEach(System::setProperty);
                    if (explicit) {
                        System.setProperty(PREFIX + "handoffDelayMillis", "0");
                        System.setProperty(PREFIX + "retainStalePublication", "false");
                    } else {
                        System.clearProperty(PREFIX + "handoffDelayMillis");
                        System.clearProperty(PREFIX + "retainStalePublication");
                    }
                    return DiagnosticConfiguration.configuredOwner();
                } finally { DiagnosticStartupConfigurationScenario.restore(previous); }
            }, 0, false);
        } finally { DiagnosticStartupConfigurationScenario.restore(previous); }
    }
}
