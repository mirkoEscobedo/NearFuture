import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import jdk.jfr.consumer.RecordedEvent;
import jdk.jfr.consumer.RecordingFile;

/** Offline bounded runtime provenance from the owned JFR, never configured launcher guesses. */
public final class RuntimeEvidence {
    private RuntimeEvidence() { }
    public static void main(String[] args) {
        try {
            if (args.length != 4 || !args[1].matches("[a-f0-9]{32}")
                || !args[2].matches("[a-f0-9]{64}") || !args[3].matches("[1-9][0-9]{0,15}")) throw new IllegalArgumentException();
            long expectedPid = Long.parseLong(args[3]);
            Path path = Path.of(args[0]);
            if (Files.size(path) > 67_108_864) throw new IllegalArgumentException();
            String flagsHash = null, versionHash = null;
            int configurationCount = 0, presentations = 0;
            boolean observer = false;
            long scanned = 0;
            try (RecordingFile file = new RecordingFile(path)) {
                while (file.hasMoreEvents()) {
                    if (++scanned > 1_000_000) throw new IllegalArgumentException();
                    RecordedEvent event = file.readEvent();
                    switch (event.getEventType().getName()) {
                        case "nf.FrameProbeRejected", "jdk.DataLoss" -> throw new IllegalArgumentException();
                        case "jdk.JVMInformation" -> {
                            if (event.getLong("pid") != expectedPid) throw new IllegalArgumentException();
                            String arguments = bounded(event.getString("jvmArguments"), false);
                            String flags = bounded(event.getString("jvmFlags"), true);
                            if (!arguments.contains("-javaagent:") || !arguments.contains("=" + args[2])) throw new IllegalArgumentException();
                            String currentFlags = digest(arguments, flags);
                            String currentVersion = digest(bounded(event.getString("jvmName"), false), bounded(event.getString("jvmVersion"), false));
                            if (flagsHash != null && (!flagsHash.equals(currentFlags) || !versionHash.equals(currentVersion))) throw new IllegalArgumentException();
                            flagsHash = currentFlags; versionHash = currentVersion;
                        }
                        case "nf.ProbeConfiguration" -> {
                            if (++configurationCount != 1 || !args[1].equals(event.getString("traceId"))
                                || !args[2].equals(event.getString("classSha256"))) throw new IllegalArgumentException();
                            observer = event.getBoolean("observerEnabled");
                        }
                        case "nf.PresentInterval" -> {
                            if (++presentations > 100_000 || !args[1].equals(event.getString("traceId"))
                                || event.getLong("frameId") != presentations || event.getLong("intervalNanos") < 1000
                                || event.getLong("clockUncertaintyNanos") < 0 || event.getThread() == null
                                || event.getThread().getJavaThreadId() <= 0) throw new IllegalArgumentException();
                        }
                        default -> { }
                    }
                }
            }
            if (flagsHash == null || configurationCount != 1 || presentations == 0) throw new IllegalArgumentException();
            System.out.println("{\"schemaVersion\":1,\"traceId\":\"" + args[1]
                + "\",\"classSha256\":\"" + args[2] + "\",\"observerEnabled\":" + observer
                + ",\"pid\":" + expectedPid + ",\"liveJvmFlagsSha256\":\"" + flagsHash
                + "\",\"runtimeVersionSha256\":\"" + versionHash + "\",\"presentations\":" + presentations + "}");
        } catch (Exception failure) {
            System.err.println("RUNTIME_EVIDENCE_UNAVAILABLE");
            System.exit(1);
        }
    }
    private static String bounded(String value, boolean nullable) {
        if (value == null && nullable) return "";
        if (value == null || value.length() > 65_536 || (!nullable && value.isEmpty())) throw new IllegalArgumentException();
        return value;
    }
    /** Ordered UTF-8 fields, each prefixed by an unsigned four-byte big-endian byte count. */
    private static String digest(String first, String second) throws Exception {
        MessageDigest digest = MessageDigest.getInstance("SHA-256");
        for (String value : new String[] { first, second }) {
            byte[] bytes = value.getBytes(StandardCharsets.UTF_8);
            digest.update(ByteBuffer.allocate(4).putInt(bytes.length).array()); digest.update(bytes);
        }
        return HexFormat.of().formatHex(digest.digest());
    }
}
