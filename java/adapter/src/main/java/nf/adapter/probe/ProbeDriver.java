package nf.adapter.probe;

import com.fs.starfarer.api.EveryFrameScript;
import com.fs.starfarer.api.Global;
import com.fs.starfarer.api.campaign.SectorAPI;
import java.io.IOException;
import java.nio.file.Path;
import java.util.HexFormat;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.BooleanSupplier;
import nf.probe.OwnedProbeFactory;
import nf.probe.ProbeMailbox;
import nf.probe.ProbeEvidence;
import nf.probe.ProbeRequest;
import nf.probe.ProbeResult;
import nf.probe.ProbeService;
import nf.probe.ProbeTransport;

/** Optional implementation. No class from this driver is linked by the base NearFuture plugin.
 * Public game APIs stay on one callback owner; JDK transport/process work stays on its worker. */
final class ProbeDriver implements ProbeHooks {
    private final DeferredFactory factory = new DeferredFactory();
    private final ProbeEvidence evidence = new ProbeEvidence(true);
    private final ProbeService service = new ProbeService(factory, evidence);
    private final AtomicReference<Thread> callback = new AtomicReference<>();
    private transient SectorAPI sector;
    private transient Script script;
    private long generation;
    private volatile String state = "STARTING";

    static ProbeHooks start() { return new ProbeDriver(); }
    private boolean callbackAllowed() {
        Thread caller = Thread.currentThread();
        callback.compareAndSet(null, caller);
        if (caller == callback.get()) return true;
        service.shutdown(); state = "CALLBACK_THREAD_UNAVAILABLE"; return false;
    }
    @Override public void resume() {
        if (!callbackAllowed()) return;
        suspend();
        if (generation == Long.MAX_VALUE || service.isStopped()) { state = "UNAVAILABLE"; return; }
        SectorAPI current = Global.getSector();
        if (current == null) return;
        try {
            ProbeMailbox mailbox = service.activate(++generation);
            Script next = new Script(mailbox);
            current.addTransientScript(next);
            script = next; sector = current; state = "BOUND";
        } catch (RuntimeException | LinkageError unavailable) { service.suspend(); state = "BIND_UNAVAILABLE"; }
    }
    @Override public void suspend() {
        if (!callbackAllowed()) return;
        service.suspend();
        if (script != null) {
            script.mailbox.revoke();
            try { sector.removeTransientScript(script); }
            finally { script = null; sector = null; }
        }
        state = "SUSPENDED";
    }
    @Override public String status() { return state + ":" + factory.phase + (evidence.unavailable() ? ":EVIDENCE_UNAVAILABLE" : ""); }

    private final class Script implements EveryFrameScript {
        private final ProbeMailbox mailbox;
        private long sequence;
        private int frames;
        private boolean pending;
        Script(ProbeMailbox mailbox) { this.mailbox = mailbox; }
        @Override public boolean isDone() { return !mailbox.active(); }
        @Override public boolean runWhilePaused() { return true; }
        @Override public void advance(float amount) {
            long begin = System.nanoTime();
            boolean sample = false;
            try {
            if (!callbackAllowed() || isDone()) return;
            ProbeResult result = mailbox.poll();
            if (result != null) {
                sample = true;
                pending = false; state = result.status().name(); frames = 120;
                if (result.status() != ProbeResult.Status.REPLIED) sequence = 8;
            }
            // At most eight application requests per binding; no retry after an unavailable host.
            if (pending || frames-- > 0 || sequence >= 8) return;
            byte[] id = new byte[16], challenge = new byte[32];
            long next = ++sequence;
            for (int i = 0; i < 8; i++) {
                id[i] = (byte)(mailbox.generation() >>> (56 - i * 8));
                id[i + 8] = (byte)(next >>> (56 - i * 8));
            }
            System.arraycopy(id, 0, challenge, 0, 16);
            System.arraycopy(id, 0, challenge, 16, 16);
            pending = mailbox.submit(id, challenge, 5000);
            sample = pending;
            } finally { if (sample) mailbox.callbackFinished(System.nanoTime() - begin); }
        }
    }

    /** Even configuration/path/hex parsing is deferred to the worker's first open. */
    private static final class DeferredFactory implements ProbeTransport.Factory {
        private OwnedProbeFactory delegate;
        private volatile String phase = "NOT_STARTED";
        @Override public ProbeTransport open(ProbeRequest request, BooleanSupplier cancelled) throws IOException {
            phase = "CONFIGURE";
            try {
                if (delegate == null) {
                    String executable = System.getProperty("nf.probe.executable");
                    String pin = System.getProperty("nf.probe.sha256");
                    if (executable == null || pin == null || !pin.matches("[a-fA-F0-9]{64}"))
                        throw new IOException("Probe configuration unavailable");
                    delegate = new OwnedProbeFactory(Path.of(executable), HexFormat.of().parseHex(pin));
                }
                phase = "OPEN";
                ProbeTransport transport = delegate.open(request, cancelled);
                phase = "AUTHENTICATED";
                return transport;
            } catch (IOException | RuntimeException | LinkageError unavailable) {
                phase = delegate == null ? "CONFIGURE_UNAVAILABLE" : delegate.phase().name() + "_UNAVAILABLE";
                throw unavailable;
            }
        }
        @Override public void close() throws IOException {
            try { if (delegate != null) delegate.close(); }
            finally { phase = delegate == null ? "STOPPED" : delegate.phase().name(); }
        }
    }
}
