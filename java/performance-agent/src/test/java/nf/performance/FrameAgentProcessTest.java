package nf.performance;

import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.concurrent.TimeUnit;

/** Actual -javaagent process entry; expected markers are independent of the transformer. */
public final class FrameAgentProcessTest {
    private FrameAgentProcessTest() { }
    public static void main(String[] args) throws Exception {
        byte[] fixture;
        try (var input = FrameAgentProcessTest.class.getResourceAsStream("/org/lwjgl/opengl/Display.class")) {
            if (input == null) throw new AssertionError("compiled fixture required");
            fixture = input.readAllBytes();
        }
        String digest = HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(fixture));
        exercise(args, digest, "admitted");
        exercise(args, "00".repeat(32), "wrong-pin");
        exercise(args, digest, "malformed");
        exercise(args, digest, "disabled");
        exercise(args, digest, "gap");
        exercise(args, digest, "wrong-thread");
        exercise(args, digest, "silent-gap");
        exercise(args, digest, "startup");
        System.out.println("PROBE_FIXTURE_JAVA=" + System.getProperty("java.runtime.version"));
    }
    private static void exercise(String[] args, String digest, String mode) throws Exception {
        Path recording = Files.createTempFile("nf-presentation-fixture-", ".jfr");
        try {
            String javaExecutable = Path.of(System.getProperty("java.home"), "bin", "java").toString();
            var command = new java.util.ArrayList<String>();
            command.add(javaExecutable);
            if (mode.equals("startup")) {
                Files.delete(recording); // This empty tempfile was created exclusively by this fixture.
                command.add("-Xlog:jfr+startup=warning");
                command.add("-XX:StartFlightRecording=name=NF100,settings=default,duration=15s,maxsize=4m,dumponexit=true,filename=" + recording);
            }
            if (!mode.equals("disabled")) {
                String configuration = mode.equals("malformed") ? "invalid" : digest;
                command.add("-javaagent:" + args[0] + "=" + configuration);
            }
            String fixture = switch (mode) {
                case "gap", "silent-gap" -> "nf.performance.PresentationGapFixture";
                case "wrong-thread" -> "nf.performance.PresentationThreadFixture";
                case "startup" -> "nf.performance.StartupPresentationFixture";
                default -> "nf.performance.PresentationFixture";
            };
            command.addAll(java.util.List.of("-cp", args[1], fixture,
                recording.toString(), mode));
            ProcessBuilder builder = new ProcessBuilder(command).redirectErrorStream(true);
            builder.environment().put("NF_PERFORMANCE_TRACE_ID", "0123456789abcdef0123456789abcdef");
            builder.environment().put("NF_PERFORMANCE_OBSERVER", "false");
            Process child = builder.start();
            try {
                if (!child.waitFor(20, TimeUnit.SECONDS)) throw new AssertionError("probe fixture exceeded deadline");
                String output = new String(child.getInputStream().readAllBytes(), java.nio.charset.StandardCharsets.UTF_8);
                if (child.exitValue() != 0) throw new AssertionError("presentation probe missing: " + output);
                String expected = mode.equals("admitted") ? "PRESENTATION_INTERVALS_OK" : "PRESENTATION_UNOBSERVED_OK";
                if (mode.equals("gap") || mode.equals("silent-gap")) expected = "PRESENTATION_GAP_REVOKED_OK";
                if (mode.equals("wrong-thread")) expected = "PRESENTATION_THREAD_REVOKED_OK";
                if (mode.equals("startup")) expected = "STARTUP_PRESENTATION_OK";
                if (mode.equals("wrong-pin") || mode.equals("malformed")) {
                    expected = "NF_FRAME_PROBE_REJECTED\n" + expected;
                }
                String normalized = output.replace("\r\n", "\n");
                if (mode.equals("startup")) {
                    // HotSpot installs informational startup logging after parsing -Xlog.
                    normalized = normalized.lines().filter(line -> !line.matches("\\[[0-9.]+s\\]\\[info\\]\\[jfr,startup\\].*") )
                        .collect(java.util.stream.Collectors.joining("\n"));
                }
                if (!normalized.trim().equals(expected)) throw new AssertionError("unexpected probe output: " + output);
                if (mode.equals("startup")) {
                    Process evidence = new ProcessBuilder(javaExecutable, "-cp", args[1], "RuntimeEvidence",
                        recording.toString(), "0123456789abcdef0123456789abcdef", digest, Long.toString(child.pid()))
                        .redirectErrorStream(true).start();
                    try {
                        if (!evidence.waitFor(10, TimeUnit.SECONDS)) throw new AssertionError("runtime reader exceeded deadline");
                        String value = new String(evidence.getInputStream().readAllBytes(), java.nio.charset.StandardCharsets.UTF_8);
                        if (evidence.exitValue() != 0) throw new AssertionError("valid owned runtime evidence missing: " + value);
                        if (!value.contains("\"pid\":" + child.pid()) || !value.contains("\"observerEnabled\":false")
                            || !value.contains("\"classSha256\":\"" + digest + "\"")) throw new AssertionError("owned runtime identity mismatch");
                        if (value.contains(args[0]) || value.contains(recording.toString())) throw new AssertionError("private runtime path exported");
                    } finally {
                        if (evidence.isAlive()) evidence.destroyForcibly();
                        if (!evidence.waitFor(2, TimeUnit.SECONDS)) throw new AssertionError("runtime reader not reaped");
                    }
                }
            } finally {
                if (child.isAlive()) child.destroyForcibly();
                if (!child.waitFor(2, TimeUnit.SECONDS)) throw new AssertionError("probe fixture not reaped");
            }
        } finally { Files.deleteIfExists(recording); }
    }
}
