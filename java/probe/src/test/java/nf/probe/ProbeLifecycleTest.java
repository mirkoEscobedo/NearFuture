package nf.probe;

import java.io.IOException;
import java.util.Arrays;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

/** Real worker/barrier tests. No game mocks can qualify loader or native save exclusion. */
public final class ProbeLifecycleTest {
    private static final byte[] SESSION = bytes(16, 5);
    private static final byte[] ID = bytes(16, 1);
    private static final byte[] CHALLENGE = bytes(32, 9);
    private ProbeLifecycleTest() { }

    public static void main(String[] args) throws Exception {
        immutableAndBounded();
        cancellationAndStaleGeneration();
        workerIsolationAndLoss();
        malformedReplyAndCloseFailure();
        failedStartupCleanupIsSticky();
        forceAttemptSurvivesFailedReap();
        System.out.println("Probe lifecycle: 6 groups PASS; no native lifecycle certification.");
    }
    private static void immutableAndBounded() throws Exception {
        ProbeMailbox mailbox = new ProbeMailbox(1);
        byte[] id = ID.clone(), challenge = CHALLENGE.clone();
        check(mailbox.submit(id, challenge, 5000), "submit");
        check(!mailbox.submit(ID, CHALLENGE, 5000), "one outstanding");
        id[0] = 99; challenge[0] = 99;
        ProbeRequest request = mailbox.take();
        check(Arrays.equals(ID, request.id()) && Arrays.equals(CHALLENGE, request.challenge()), "copy input");
        request.challenge()[0] = 99;
        check(Arrays.equals(CHALLENGE, request.challenge()), "copy output");
        check(!mailbox.complete(request, new ProbeResult(2, ID, SESSION, ProbeResult.Status.REPLIED, CHALLENGE)), "generation fence");
        check(mailbox.complete(request, reply(request)), "complete");
        check(!mailbox.complete(request, reply(request)), "one completion");
        ProbeResult result = mailbox.poll();
        result.payload()[0] = 99;
        check(Arrays.equals(CHALLENGE, result.payload()) && mailbox.poll() == null, "immutable single reply");
        check(mailbox.submit(ID, CHALLENGE, 1), "short deadline");
        Thread.sleep(5);
        check(mailbox.poll().status() == ProbeResult.Status.DEADLINE, "deadline without worker");
        rejects(() -> mailbox.submit(new byte[15], CHALLENGE, 100));
        rejects(() -> mailbox.submit(ID, new byte[33], 100));
        rejects(() -> mailbox.submit(ID, CHALLENGE, 5001));
        rejects(() -> new ProbeResult(1, ID, new byte[16], ProbeResult.Status.REPLIED, CHALLENGE));
    }
    private static void cancellationAndStaleGeneration() {
        ProbeMailbox mailbox = new ProbeMailbox(1);
        check(mailbox.submit(ID, CHALLENGE, 5000), "submit old");
        ProbeRequest old = mailbox.take();
        mailbox.cancel();
        byte[] nextId = bytes(16, 2);
        check(mailbox.submit(nextId, CHALLENGE, 5000), "submit successor");
        ProbeRequest next = mailbox.take();
        check(!mailbox.complete(old, reply(old)), "cancelled completion refused");
        check(mailbox.complete(next, reply(next)), "successor completion intact");
        check(Arrays.equals(nextId, mailbox.poll().id()), "successor ID");
        mailbox.revoke();
        check(!mailbox.complete(next, reply(next)) && !mailbox.submit(ID, CHALLENGE, 5000)
                && mailbox.poll() == null, "revocation");
    }
    private static void workerIsolationAndLoss() throws Exception {
        Thread callback = Thread.currentThread();
        CountDownLatch entered = new CountDownLatch(1), release = new CountDownLatch(1), closed = new CountDownLatch(1);
        AtomicInteger opens = new AtomicInteger();
        AtomicReference<Throwable> failure = new AtomicReference<>();
        ProbeService service = new ProbeService((request, cancelled) -> {
            try { check(Thread.currentThread() != callback, "factory worker"); }
            catch (Throwable problem) { failure.set(problem); }
            if (opens.incrementAndGet() > 1) throw new IOException("absent host");
            return new ProbeTransport() {
                @Override public ProbeResult exchange(ProbeRequest value, java.util.function.BooleanSupplier revoked) {
                    entered.countDown();
                    try { check(release.await(2, TimeUnit.SECONDS), "test release"); }
                    catch (InterruptedException interrupted) { throw new AssertionError(interrupted); }
                    check(revoked.getAsBoolean(), "old work cancellation visible");
                    return reply(value); // Deliberately late; must never reach the new binding.
                }
                @Override public void close() { check(Thread.currentThread() != callback, "close worker"); closed.countDown(); }
            };
        });
        try {
            ProbeMailbox old = service.activate(1);
            check(old.submit(ID, CHALLENGE, 5000), "worker request");
            check(entered.await(2, TimeUnit.SECONDS), "worker entered");
            // This callback must finish while the transport is held by a barrier.
            service.suspend();
            ProbeMailbox next = service.activate(2);
            check(!old.active() && next.submit(ID, CHALLENGE, 5000), "new binding before I/O returns");
            release.countDown();
            check(closed.await(2, TimeUnit.SECONDS), "owned connection closed");
            check(await(next).status() == ProbeResult.Status.UNAVAILABLE, "host loss is a value");
            check(old.poll() == null && opens.get() == 2, "stale discarded; no retry loop");
            check(failure.get() == null, "worker isolation");
            rejectsState(() -> service.activate(2));
        } finally { release.countDown(); stop(service); }
    }
    private static void malformedReplyAndCloseFailure() throws Exception {
        for (boolean closeFails : new boolean[]{false, true}) {
            ProbeService service = new ProbeService((request, cancelled) -> new ProbeTransport() {
                @Override public ProbeResult exchange(ProbeRequest value, java.util.function.BooleanSupplier revoked) {
                    return new ProbeResult(value.generation(), value.id(), SESSION, ProbeResult.Status.REPLIED,
                            closeFails ? value.challenge() : bytes(32, 88));
                }
                @Override public void close() throws IOException { if (closeFails) throw new IOException("close failed"); }
            });
            try {
                ProbeMailbox mailbox = service.activate(1);
                check(mailbox.submit(ID, CHALLENGE, 5000), "failure request");
                check(await(mailbox).status() == (closeFails ? ProbeResult.Status.UNAVAILABLE : ProbeResult.Status.REJECTED), "fail closed");
            } finally { stop(service); }
        }
    }
    private static ProbeResult await(ProbeMailbox mailbox) throws Exception {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3);
        while (System.nanoTime() - deadline < 0) {
            ProbeResult result = mailbox.poll();
            if (result != null) return result;
            Thread.sleep(2);
        }
        throw new AssertionError("Missing bounded result");
    }
    private static void forceAttemptSurvivesFailedReap() throws Exception {
        AtomicInteger recorded = new AtomicInteger(), destroyed = new AtomicInteger();
        Process process = new Process() {
            @Override public java.io.OutputStream getOutputStream() { return java.io.OutputStream.nullOutputStream(); }
            @Override public java.io.InputStream getInputStream() { return java.io.InputStream.nullInputStream(); }
            @Override public java.io.InputStream getErrorStream() { return java.io.InputStream.nullInputStream(); }
            @Override public int waitFor() { throw new AssertionError("Unbounded wait"); }
            @Override public boolean waitFor(long timeout, TimeUnit unit) { return false; }
            @Override public int exitValue() { throw new IllegalThreadStateException(); }
            @Override public void destroy() { throw new AssertionError("Wrong stop operation"); }
            @Override public Process destroyForcibly() {
                check(recorded.get() > 0, "force recorded before attempted action"); destroyed.incrementAndGet(); return this;
            }
        };
        try { OwnedProbeHost.stop(process, recorded::incrementAndGet); throw new AssertionError("Unknown reap accepted"); }
        catch (IOException expected) { /* force attempt remains distinct from terminal proof */ }
        check(recorded.get() == 1 && destroyed.get() == 1, "force fact survives exceptional reap");
    }
    private static void failedStartupCleanupIsSticky() throws Exception {
        AtomicInteger starts = new AtomicInteger();
        OwnedProbeFactory factory = new OwnedProbeFactory(java.nio.file.Path.of(System.getProperty("java.home")).toAbsolutePath(),
                new byte[32], (path, pin, request, cancelled) -> {
                    starts.incrementAndGet();
                    throw new OwnedProbeHost.CleanupFailure(null, new IOException("injected unknown reap"));
                });
        ProbeService service = new ProbeService(factory);
        try {
            for (long generation = 1; generation <= 2; generation++) {
                ProbeMailbox mailbox = service.activate(generation);
                check(mailbox.submit(ID, CHALLENGE, 5000), "cleanup-failed request");
                check(await(mailbox).status() == ProbeResult.Status.UNAVAILABLE, "cleanup-failed value");
            }
            check(starts.get() == 1 && factory.phase() == OwnedProbeFactory.Phase.CLEANUP_FAILED,
                    "no successor after failed-start cleanup");
        } finally { stop(service); }
        check(service.cleanupFailed() && factory.phase() == OwnedProbeFactory.Phase.CLEANUP_FAILED,
                "final close preserves failed cleanup history");
    }
    private static void stop(ProbeService service) throws Exception {
        service.shutdown();
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3);
        while (!service.isStopped() && System.nanoTime() - deadline < 0) Thread.sleep(2);
        check(service.isStopped(), "worker stopped");
    }
    private static ProbeResult reply(ProbeRequest request) {
        return new ProbeResult(request.generation(), request.id(), SESSION, ProbeResult.Status.REPLIED, request.challenge());
    }
    private static byte[] bytes(int size, int first) { byte[] value = new byte[size]; value[0] = (byte) first; return value; }
    private static void rejects(Runnable call) {
        try { call.run(); } catch (IllegalArgumentException expected) { return; }
        throw new AssertionError("Expected validation refusal");
    }
    private static void rejectsState(Runnable call) {
        try { call.run(); } catch (IllegalStateException expected) { return; }
        throw new AssertionError("Expected generation refusal");
    }
    private static void check(boolean value, String reason) { if (!value) throw new AssertionError(reason); }
}
