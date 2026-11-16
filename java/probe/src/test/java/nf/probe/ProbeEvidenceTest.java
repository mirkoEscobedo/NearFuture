package nf.probe;

import java.nio.file.Files;
import java.nio.file.Path;

/** Bounded observable evidence without callback I/O or payload/secret serialization. */
public final class ProbeEvidenceTest {
    private ProbeEvidenceTest() { }
    public static void main(String[] args) throws Exception {
        Path directory = Files.createTempDirectory("nf-probe-evidence-");
        Path file = directory.resolve("facts.jsonl");
        ProbeEvidence evidence = new ProbeEvidence(true);
        try {
            evidence.open(file);
            long baseline = Files.size(file);
            byte[] id = new byte[16], session = new byte[16], payload = new byte[32];
            id[0] = 1; session[0] = 2; java.util.Arrays.fill(payload, (byte)'S');
            ProbeResult reply = new ProbeResult(1, id, session, ProbeResult.Status.REPLIED, payload);
            Thread callback = new Thread(() -> {
                for (int i = 0; i < 100; i++) evidence.callback(ProbeEvidence.Kind.CALLBACK_CONSUMED, 1, reply, null, 17);
            });
            callback.start(); callback.join(2000);
            check(!callback.isAlive() && Files.size(file) == baseline, "callback did not write or wait");
            evidence.drain();
            for (int i = 0; i < 300; i++) evidence.worker(ProbeEvidence.Kind.REQUEST, 1, null, null);
            evidence.close();
            String text = Files.readString(file);
            check(text.lines().count() == 258 && Files.size(file) < 131072, "hard record/byte bound");
            check(text.contains("\"records\":256,\"dropped\":144") && text.contains("\"callbackNanos\":17"), "truthful drops and duration");
            check(text.contains("CALLBACK_CONSUMED") && !text.contains("SSSSSSSS") && !text.contains("payload")
                    && !text.contains("token"), "closed facts only");
            check(!evidence.unavailable(), "writer closed");
            try { new ProbeEvidence(true).open(file); throw new AssertionError("Original evidence overwritten"); }
            catch (java.io.IOException expected) { /* create-new preserves originals */ }
        } finally { evidence.close(); Files.deleteIfExists(file); Files.deleteIfExists(directory); }
        optionalLinkageFailureIsContained();
        System.out.println("Probe evidence: callback isolation, cap/drops, redaction, create-new and optional linkage failure isolation PASS.");
    }
    private static void optionalLinkageFailureIsContained() throws Exception {
        ProbeEvidence broken = new ProbeEvidence(new java.io.BufferedWriter(java.io.Writer.nullWriter()) {
            @Override public void write(String value) { throw new NoClassDefFoundError("fixture-formatting"); }
            @Override public void close() { throw new NoClassDefFoundError("fixture-close"); }
        });
        ProbeService service = new ProbeService((request, cancelled) -> new ProbeTransport() {
            @Override public ProbeResult exchange(ProbeRequest request, java.util.function.BooleanSupplier revoked) {
                byte[] session = new byte[16]; session[0] = 9;
                return new ProbeResult(request.generation(), request.id(), session, ProbeResult.Status.REPLIED, request.challenge());
            }
            @Override public void close() { }
        }, broken);
        try {
            ProbeMailbox mailbox = service.activate(1);
            byte[] id = new byte[16]; id[0] = 1;
            check(mailbox.submit(id, new byte[32], 1000), "isolation request");
            ProbeResult result = null;
            long end = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(2);
            while (result == null && System.nanoTime() - end < 0) { result = mailbox.poll(); Thread.sleep(2); }
            check(result != null && result.status() == ProbeResult.Status.REPLIED && broken.unavailable(),
                    "evidence linkage cannot kill protocol delivery");
        } finally {
            service.shutdown();
            long end = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(2);
            while (!service.isStopped() && System.nanoTime() - end < 0) Thread.sleep(2);
            check(service.isStopped(), "evidence close linkage cannot hide terminal worker");
        }
    }
    private static void check(boolean value, String reason) { if (!value) throw new AssertionError(reason); }
}
