package nf.probe;

import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.HexFormat;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;

/** Opt-in, bounded private evidence. Only the service worker opens/writes/closes the file.
 * One immutable callback slot; a busy slot drops rather than waits. No peer text or payload. */
public final class ProbeEvidence implements AutoCloseable {
    enum Kind { BIND, SUSPEND, REQUEST, WORKER_RESULT, CALLBACK_CONSUMED, CALLBACK_FRAME }
    private record Fact(Kind kind, long generation, byte[] id, byte[] session,
                        ProbeResult.Status status, long duration, long at) { }
    private final boolean configured;
    private final AtomicReference<Fact> callback = new AtomicReference<>();
    private final AtomicLong dropped = new AtomicLong();
    private BufferedWriter writer;
    private int records;
    private volatile boolean unavailable;
    private volatile boolean enabled;

    public ProbeEvidence(boolean configured) { this.configured = configured; }
    ProbeEvidence(BufferedWriter ownedWriter) { this.configured = false; this.writer = ownedWriter; this.enabled = true; }
    /** Worker only; the path is an explicit private destination, never a discovery/credential file. */
    void open() {
        if (!configured) return;
        try {
            String destination = System.getProperty("nf.probe.evidence");
            if (destination == null) return;
            open(Path.of(destination));
        }
        catch (IOException | RuntimeException | LinkageError failure) { unavailable = true; }
    }
    void open(Path destination) throws IOException {
        if (!destination.isAbsolute()) throw new IOException("Private probe evidence requires absolute path");
        writer = Files.newBufferedWriter(destination, StandardCharsets.US_ASCII,
                StandardOpenOption.CREATE_NEW, StandardOpenOption.WRITE);
        writer.write("{\"format\":1,\"kind\":\"OPEN\",\"maxRecords\":256,\"callbackSlots\":1}\n"); writer.flush();
        enabled = true;
    }
    public boolean unavailable() { return unavailable; }
    boolean enabled() { return enabled; }
    void callback(Kind kind, long generation, ProbeResult result, ProbeRequest request, long duration) {
        try {
            if (!enabled) return;
            Fact fact = fact(kind, generation, result, request, duration);
            if (!callback.compareAndSet(null, fact)) dropped.incrementAndGet();
        } catch (RuntimeException | LinkageError failure) { unavailable = true; enabled = false; }
    }
    void worker(Kind kind, long generation, ProbeResult result, ProbeRequest request) {
        try { if (enabled) write(fact(kind, generation, result, request, 0)); }
        catch (RuntimeException | LinkageError failure) { unavailable = true; enabled = false; }
    }
    private static Fact fact(Kind kind, long generation, ProbeResult result, ProbeRequest request, long duration) {
        return new Fact(kind, generation, result != null ? result.id() : request != null ? request.id() : new byte[16],
                result == null ? new byte[16] : result.serviceSession(), result == null ? null : result.status(),
                Math.max(0, duration), System.nanoTime());
    }
    void drain() { Fact fact = callback.getAndSet(null); if (fact != null && enabled) write(fact); }
    private void write(Fact fact) {
        try {
        if (records == 256) { dropped.incrementAndGet(); return; }
        records++;
        String line = "{\"format\":1,\"event\":" + records + ",\"kind\":\"" + fact.kind().name()
                + "\",\"generation\":" + fact.generation() + ",\"requestId\":\"" + HexFormat.of().formatHex(fact.id())
                + "\",\"serviceSession\":\"" + HexFormat.of().formatHex(fact.session()) + "\",\"status\":\""
                + (fact.status() == null ? "NONE" : fact.status().name()) + "\",\"callbackNanos\":" + fact.duration()
                + ",\"atNanos\":" + fact.at() + ",\"outstandingLimit\":1,\"echoVerified\":"
                + (fact.status() == ProbeResult.Status.REPLIED) + "}\n";
        writer.write(line); writer.flush();
        } catch (IOException | RuntimeException | LinkageError failure) { unavailable = true; enabled = false; }
    }
    @Override public void close() {
        if (writer == null) return;
        enabled = false;
        Fact last = callback.getAndSet(null);
        if (last != null) write(last);
        BufferedWriter closing = writer;
        writer = null;
        try {
            closing.write("{\"format\":1,\"kind\":\"CLOSED\",\"records\":" + records + ",\"dropped\":" + dropped.get()
                    + ",\"countsAreSnapshot\":true,\"unavailable\":" + unavailable + "}\n");
        } catch (IOException | RuntimeException | LinkageError failure) { unavailable = true; }
        finally {
            try { closing.close(); } catch (IOException | RuntimeException | LinkageError failure) { unavailable = true; }
        }
    }
}
