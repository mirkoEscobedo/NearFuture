package nf.probe;

import java.io.IOException;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.concurrent.TimeUnit;

/** Explicit actual Java-parent/Rust-child interoperability; never launches Starsector. */
public final class ProbeProcessTest {
    private ProbeProcessTest() { }
    public static void main(String[] args) throws Exception {
        if (args.length != 2) throw new IllegalArgumentException("Expected executable and SHA256 pin");
        Path executable = Path.of(args[0]);
        byte[] pin = HexFormat.of().parseHex(args[1]);
        byte[] firstSession;
        ProbeRequest one = request(1, 1);
        OwnedProbeHost owned = OwnedProbeHost.start(executable, pin, 15000, one, () -> false);
        long pid = owned.pid();
        try {
            owned.bind(one, () -> false);
            firstSession = owned.serviceSession();
            try (ModuleProbeClient client = owned.connect(one, () -> false)) {
                success(client.exchange(one, () -> false), one);
                ProbeRequest two = request(2, 2);
                owned.bind(two, () -> false);
                try { client.exchange(one, () -> false); throw new AssertionError("Old connection survived rebind"); }
                catch (IOException expected) { /* parent acknowledgment fences before a new connection */ }
                try (ModuleProbeClient current = owned.connect(two, () -> false)) {
                    success(current.exchange(two, () -> false), two);
                    check(Arrays.equals(firstSession, owned.serviceSession()), "service survives campaign replacement");
                    ProbeResult stale = current.exchange(request(1, 3), () -> false);
                    check(stale.status() == ProbeResult.Status.REJECTED, "authenticated stale generation refusal");
                }
            }
        } finally { owned.close(); }
        check(!owned.alive() && !ProcessHandle.of(pid).map(ProcessHandle::isAlive).orElse(false), "owned child reaped");
        check(!owned.forcedCleanup(), "ordinary parent EOF cleanup");

        ProbeRequest restart = request(1, 4);
        try (OwnedProbeHost next = OwnedProbeHost.start(executable, pin, 15000, restart, () -> false)) {
            check(!Arrays.equals(firstSession, next.serviceSession()), "fresh service session on restart");
            next.bind(restart, () -> false);
            try (ModuleProbeClient client = next.connect(restart, () -> false)) {
                try { client.exchange(restart, () -> true); throw new AssertionError("Cancelled request sent"); }
                catch (IOException expected) { /* no accepted result */ }
            }
        }
        try { OwnedProbeHost.start(executable, new byte[32], 15000, request(1, 5), () -> false);
            throw new AssertionError("Wrong executable pin accepted"); }
        catch (IOException expected) { /* refusal precedes child creation */ }

        OwnedProbeFactory factory = new OwnedProbeFactory(executable, pin);
        java.nio.file.Path evidenceDirectory = java.nio.file.Files.createTempDirectory("nf-probe-actual-");
        java.nio.file.Path evidencePath = evidenceDirectory.resolve("facts.jsonl");
        String previousEvidence = System.getProperty("nf.probe.evidence");
        System.setProperty("nf.probe.evidence", evidencePath.toString());
        ProbeEvidence evidence = new ProbeEvidence(true);
        ProbeService service = new ProbeService(factory, evidence);
        try {
            ProbeMailbox mailbox = service.activate(1);
            check(mailbox.submit(bytes(16, 6), bytes(32, 7), 5000), "actual worker submit");
            ProbeResult first = await(mailbox);
            long firstPid = factory.childPid();
            ProbeMailbox second = service.activate(2);
            check(!mailbox.active() && second.submit(bytes(16, 8), bytes(32, 9), 5000), "actual worker rebind");
            ProbeResult next = await(second);
            check(firstPid == factory.childPid() && Arrays.equals(first.serviceSession(), next.serviceSession()),
                    "one actual service lifetime across campaign generations");
        } finally {
            service.shutdown();
            long end = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
            while (!service.isStopped() && System.nanoTime() - end < 0) Thread.sleep(2);
            check(service.isStopped(), "actual worker terminal");
            check(!service.cleanupFailed() && factory.phase() == OwnedProbeFactory.Phase.STOPPED
                    && !ProcessHandle.of(factory.childPid()).map(ProcessHandle::isAlive).orElse(false), "actual worker child reaped");
            if (previousEvidence == null) System.clearProperty("nf.probe.evidence");
            else System.setProperty("nf.probe.evidence", previousEvidence);
        }
        String facts = java.nio.file.Files.readString(evidencePath);
        check(!evidence.unavailable() && facts.contains("WORKER_RESULT") && facts.contains("CALLBACK_CONSUMED")
                && facts.contains("\"generation\":2") && facts.contains("\"echoVerified\":true"), "actual observable reply/consumption");
        java.nio.file.Files.delete(evidencePath); java.nio.file.Files.delete(evidenceDirectory);
        System.out.println("Probe actual Java/Rust: authenticated echo, rebind ACK, stale rejection, fresh session, cancellation, pin refusal, EOF/worker cleanup PASS.");
    }
    private static ProbeResult await(ProbeMailbox mailbox) throws Exception {
        ProbeResult result = null;
        long end = System.nanoTime() + TimeUnit.SECONDS.toNanos(6);
        while (result == null && System.nanoTime() - end < 0) { result = mailbox.poll(); Thread.sleep(2); }
        check(result != null && result.status() == ProbeResult.Status.REPLIED, "actual authenticated worker reply");
        mailbox.callbackFinished(1); // Deterministic callback-duration fixture; not native timing evidence.
        return result;
    }
    private static ProbeRequest request(long generation, int id) {
        return new ProbeRequest(generation, bytes(16, id), bytes(32, id), System.nanoTime() + TimeUnit.SECONDS.toNanos(5));
    }
    private static byte[] bytes(int size, int first) { byte[] result = new byte[size]; result[0] = (byte)first; return result; }
    private static void success(ProbeResult result, ProbeRequest request) {
        check(result.status() == ProbeResult.Status.REPLIED && request.matches(result)
                && Arrays.equals(result.payload(), request.challenge()), "authenticated application echo");
    }
    private static void check(boolean value, String reason) { if (!value) throw new AssertionError(reason); }
}
