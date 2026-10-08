import java.nio.file.Files;
import java.nio.file.Path;
import jdk.jfr.consumer.RecordedEvent;
import jdk.jfr.consumer.RecordedThread;
import jdk.jfr.consumer.RecordingFile;

/** Offline, streaming JFR export. Never attaches to a JVM or publishes stack/path/string fields. */
public final class ExportRecording {
    private ExportRecording() { }

    private static String kind(String name) {
        return switch (name) {
            case "nf.Frame" -> "frame";
            case "nf.CampaignCallback" -> "campaign_callback";
            case "nf.Capture" -> "capture";
            case "nf.Apply" -> "apply";
            case "nf.Encode" -> "encode";
            case "nf.Save" -> "save";
            case "nf.Load" -> "load";
            case "nf.ThreadCpu" -> "thread_cpu";
            case "jdk.ExecutionSample", "jdk.NativeMethodSample" -> "thread_running_sample";
            case "jdk.JavaMonitorEnter", "jdk.JavaMonitorWait", "jdk.ThreadPark" -> "thread_blocked";
            case "jdk.ThreadSleep" -> "thread_sleep";
            case "jdk.FileRead", "jdk.FileWrite", "jdk.SocketRead", "jdk.SocketWrite" -> "thread_io";
            case "jdk.GarbageCollection" -> "gc_cycle";
            case "jdk.GCPhasePause" -> "gc_pause";
            case "jdk.ObjectAllocationSample" -> "allocation_sample";
            default -> null;
        };
    }

    public static void main(String[] args) {
        try {
            if (args.length != 2 || !args[1].matches("[a-f0-9]{32}")) throw new IllegalArgumentException();
            Path path = Path.of(args[0]);
            if (Files.size(path) > 67_108_864) throw new IllegalArgumentException();
            long scanned = 0, exported = 0;
            try (RecordingFile file = new RecordingFile(path)) {
                while (file.hasMoreEvents()) {
                    if (++scanned > 1_000_000) throw new IllegalArgumentException();
                    RecordedEvent event = file.readEvent();
                    String category = kind(event.getEventType().getName());
                    if (category == null) continue;
                    if (++exported > 100_000) throw new IllegalArgumentException();
                    if (event.hasField("traceId") && !args[1].equals(event.getString("traceId"))) throw new IllegalArgumentException();
                    long atUs = Math.addExact(Math.multiplyExact(event.getStartTime().getEpochSecond(), 1_000_000), event.getStartTime().getNano() / 1000);
                    if (atUs < 0 || atUs > 9_007_199_254_740_991L) throw new IllegalArgumentException();
                    StringBuilder out = new StringBuilder(256);
                    out.append("{\"traceId\":\"").append(args[1]).append("\",\"source\":\"jfr\",\"kind\":\"").append(category).append("\",\"atUs\":").append(atUs);
                    RecordedThread thread = category.equals("campaign_callback") ? event.getThread() :
                        event.hasField("sampledThread") ? event.getThread("sampledThread") : event.getThread();
                    if (category.equals("campaign_callback") &&
                        (!event.hasField("traceId") || thread == null || thread.getJavaThreadId() <= 0 || event.hasField("frameId")))
                        throw new IllegalArgumentException();
                    if (thread != null) out.append(",\"threadId\":").append(thread.getJavaThreadId());
                    if (!category.equals("thread_running_sample") && !category.equals("allocation_sample") && !category.equals("thread_cpu"))
                        out.append(",\"durationUs\":").append(event.getDuration().toNanos() / 1000);
                    if (category.equals("allocation_sample")) out.append(",\"bytes\":").append(event.getLong("weight"));
                    if (category.equals("thread_cpu")) out.append(",\"cpuUs\":").append(event.getLong("cpuUs"));
                    if (event.hasField("frameId")) out.append(",\"frameId\":").append(event.getLong("frameId"));
                    if (event.hasField("requestId")) {
                        String requestId = event.getString("requestId");
                        if (requestId != null && !requestId.isEmpty()) {
                            if (!requestId.matches("[a-f0-9]{32}")) throw new IllegalArgumentException();
                            out.append(",\"requestId\":\"").append(requestId).append('"');
                        }
                    }
                    out.append('}');
                    System.out.println(out);
                }
            }
        } catch (Exception error) {
            System.err.println("JFR_EXPORT_UNAVAILABLE: verify bounded recording, event fields and hexadecimal trace ID locally; discard any partial output.");
            System.exit(1);
        }
    }
}
