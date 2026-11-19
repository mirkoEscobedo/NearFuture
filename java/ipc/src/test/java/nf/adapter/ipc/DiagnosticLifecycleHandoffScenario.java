package nf.adapter.ipc;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.time.Duration;
import java.util.List;
import java.util.concurrent.TimeUnit;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordedEvent;
import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Real existing owner propagation, without game callbacks or synthetic authenticated values. */
final class DiagnosticLifecycleHandoffScenario {
    private DiagnosticLifecycleHandoffScenario() { }

    enum AuthCase { LIFECYCLE_HANDOFF, RETAINED_STALE, DEFAULT_MISSING, DEFAULT_EXPLICIT_ZERO_FALSE }
    enum Phase { INITIAL, RENEWAL, REPLACEMENT }

    static void run(NodeLaunch launch) throws InterruptedException {
        Path capture = launch.vault().getParent().resolve("lifecycle-handoff-delay.jfr");
        if (Files.exists(capture, LinkOption.NOFOLLOW_LINKS)) {
            throw new AssertionError("fresh owned lifecycle recording required");
        }
        try {
            try (Recording recording = new Recording()) {
                recording.enable("nf.DiagnosticHandoffDelay").withoutStackTrace();
                recording.setMaxSize(4L * 1024 * 1024);
                recording.start();
                LifecycleOwner owner = new LifecycleOwner(launch, new DiagnosticFaultPlan(750));
                ProcessHandle child;
                try {
                    LifecycleMailbox mailbox = owner.activate(301);
                    long epoch = authenticatedBoth(mailbox);
                    if (!mailbox.isCurrent(epoch)) throw new AssertionError("current authenticated control required");
                    List<ProcessHandle> children = ProcessHandle.current().children().filter(candidate ->
                        candidate.info().command().map(command -> Path.of(command).equals(launch.executable())).orElse(false))
                        .limit(2).toList();
                    if (children.size() != 1) throw new AssertionError("one exact authenticated lifecycle child required");
                    child = children.get(0);
                    if (!child.isAlive()) throw new AssertionError("authenticated lifecycle child must be live");
                } finally {
                    owner.shutdown();
                    long until = System.nanoTime() + TimeUnit.SECONDS.toNanos(11);
                    while (!owner.isStopped() && System.nanoTime() < until) Thread.sleep(10);
                    if (!owner.isStopped()) throw new AssertionError("lifecycle worker exceeded existing shutdown bound");
                }
                if (child.isAlive()) throw new AssertionError("exact lifecycle child survived shutdown");
                if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
                    throw new AssertionError("lifecycle handoff publication survived cleanup");
                }
                recording.stop();
                recording.dump(capture);
            }
            List<RecordedEvent> events = DiagnosticHandoffProcessScenario.delayEvents(capture);
            if (events.size() != 1) {
                throw new AssertionError("authenticated lifecycle owner: worker handoff delay event expected1, got" + events.size());
            }
            RecordedEvent event = events.get(0);
            if (event.getLong("requestedMillis") != 750 || !"COMPLETED".equals(event.getString("outcome"))
                    || event.getDuration().compareTo(Duration.ofMillis(750)) < 0) {
                throw new AssertionError("lifecycle owner must complete actual one-shot750ms handoff delay");
            }
            if (event.getThread() == null || event.getThread().getJavaThreadId() == Thread.currentThread().getId()
                    || !"nf-local-readonly-owner".equals(event.getThread().getJavaName())) {
                throw new AssertionError("real delay must run on the existing lifecycle worker");
            }
        } catch (IOException unavailable) {
            throw new AssertionError("bounded lifecycle JFR capture unavailable");
        }
    }

    private static long authenticatedBoth(LifecycleMailbox mailbox) throws InterruptedException {
        return authenticatedBoth(mailbox, 0);
    }

    static long authenticatedBoth(LifecycleMailbox mailbox, long after) throws InterruptedException {
        return authenticatedBoth(mailbox, after, AuthCase.LIFECYCLE_HANDOFF, Phase.INITIAL);
    }

    static long authenticatedBoth(LifecycleMailbox mailbox, long after, AuthCase scenario, Phase phase)
            throws InterruptedException {
        long started = System.nanoTime();
        long deadline = started + TimeUnit.SECONDS.toNanos(15);
        long observed = 0;
        boolean control = false;
        boolean bulk = false;
        LifecycleMailbox.State controlLast = null;
        LifecycleMailbox.State bulkLast = null;
        long controlSampleEpoch = 0;
        long bulkSampleEpoch = 0;
        while (System.nanoTime() < deadline) {
            long epoch = mailbox.currentEpoch();
            if (epoch != observed) { observed = epoch; control = false; bulk = false; }
            LifecycleMailbox.State controlState = mailbox.poll(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL);
            if (controlState != null) { controlLast = controlState; controlSampleEpoch = epoch; }
            control |= controlState == LifecycleMailbox.State.AUTHENTICATED;
            LifecycleMailbox.State bulkState = mailbox.poll(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK);
            if (bulkState != null) { bulkLast = bulkState; bulkSampleEpoch = epoch; }
            bulk |= bulkState == LifecycleMailbox.State.AUTHENTICATED;
            if (control && bulk && epoch > after && mailbox.isCurrent(epoch)) return epoch;
            Thread.sleep(10);
        }
        // Sequential failure observations: neither this snapshot nor a sample epoch is a producer certificate.
        long finalEpoch = mailbox.currentEpoch();
        boolean finalActive = mailbox.active();
        long elapsedMillis = TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - started);
        throw new AssertionError(authFailureContext(scenario, phase, after, observed, finalEpoch, finalActive,
            controlLast, controlSampleEpoch, control, bulkLast, bulkSampleEpoch, bulk, elapsedMillis));
    }

    static String authFailureContext(AuthCase scenario, Phase phase, long afterEpoch, long observedEpoch,
            long currentEpoch, boolean active, LifecycleMailbox.State controlLast, long controlSampleEpoch,
            boolean controlAuthSeen, LifecycleMailbox.State bulkLast, long bulkSampleEpoch,
            boolean bulkAuthSeen, long elapsedMillis) {
        java.util.Objects.requireNonNull(scenario);
        java.util.Objects.requireNonNull(phase);
        String context = "real lifecycle owner did not authenticate both current roles"
            + ";scenario=" + scenario.name() + ";phase=" + phase.name()
            + ";afterEpoch=" + afterEpoch + ";observedEpoch=" + observedEpoch
            + ";currentEpoch=" + currentEpoch + ";active=" + active
            + ";controlLast=" + (controlLast == null ? "NONE" : controlLast.name())
            + ";controlSampleEpoch=" + controlSampleEpoch + ";controlAuthSeen=" + controlAuthSeen
            + ";bulkLast=" + (bulkLast == null ? "NONE" : bulkLast.name())
            + ";bulkSampleEpoch=" + bulkSampleEpoch + ";bulkAuthSeen=" + bulkAuthSeen
            + ";elapsedMillis=" + elapsedMillis + ";finalSnapshot=NON_ATOMIC";
        if (context.length() > 512) throw new AssertionError("closed authentication failure context exceeded512 chars");
        return context;
    }
}
