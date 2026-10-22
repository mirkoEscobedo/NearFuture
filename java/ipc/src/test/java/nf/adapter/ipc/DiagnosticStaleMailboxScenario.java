package nf.adapter.ipc;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.function.Supplier;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordedEvent;
import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Retains no fake state: real authentication, renewal and revocation drive the worker seam. */
final class DiagnosticStaleMailboxScenario {
    private static final String EVENT = "nf.DiagnosticStaleMailbox";
    private DiagnosticStaleMailboxScenario() { }

    static void run(NodeLaunch launch) throws InterruptedException {
        run(launch, () -> new LifecycleOwner(launch, new DiagnosticFaultPlan(0, true)), 0);
    }

    static void run(NodeLaunch launch, Supplier<LifecycleOwner> ownerFactory, long expectedDelayMillis)
            throws InterruptedException {
        run(launch, ownerFactory, expectedDelayMillis, true);
    }

    static void run(NodeLaunch launch, Supplier<LifecycleOwner> ownerFactory, long expectedDelayMillis,
            boolean expectedStale) throws InterruptedException {
        Path before = launch.vault().getParent().resolve("stale-before-revoke.jfr");
        Path after = launch.vault().getParent().resolve("stale-after-revoke.jfr");
        if (Files.exists(before, LinkOption.NOFOLLOW_LINKS) || Files.exists(after, LinkOption.NOFOLLOW_LINKS)) {
            throw new AssertionError("fresh owned stale-mailbox recordings required");
        }
        try {
            long oldEpoch;
            long renewed;
            ProcessHandle oldChild;
            ProcessHandle newChild;
            try (Recording recording = new Recording()) {
                recording.enable(EVENT).withoutStackTrace();
                recording.enable("nf.DiagnosticHandoffDelay").withoutStackTrace();
                recording.setMaxSize(4L * 1024 * 1024);
                recording.start();
                LifecycleOwner owner = ownerFactory.get();
                if (owner == null) throw new AssertionError("enabled startup owner required");
                try {
                    LifecycleMailbox old = owner.activate(401);
                    oldEpoch = DiagnosticLifecycleHandoffScenario.authenticatedBoth(old, 0);
                    oldChild = exactChild(launch);
                    renewed = DiagnosticLifecycleHandoffScenario.authenticatedBoth(old, oldEpoch);
                    if (!old.active() || !old.isCurrent(renewed) || !oldChild.isAlive()) {
                        throw new AssertionError("natural real epoch renewal must preserve active old generation");
                    }
                    recording.dump(before);
                    if (!DiagnosticHandoffProcessScenario.events(before, EVENT).isEmpty()) {
                        throw new AssertionError("active epoch renewal released retained mailbox before revocation");
                    }
                    LifecycleMailbox current = owner.activate(402);
                    if (old.active() || old.currentEpoch() != 0
                            || old.poll(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL) != null
                            || old.poll(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK) != null) {
                        throw new AssertionError("actual generation replacement must synchronously revoke old mailbox");
                    }
                    long currentEpoch = DiagnosticLifecycleHandoffScenario.authenticatedBoth(current, 0);
                    newChild = exactChild(launch);
                    if (oldChild.isAlive() || oldChild.pid() == newChild.pid() || !current.isCurrent(currentEpoch)) {
                        throw new AssertionError("new genuine authenticated mailbox must remain independent of old release");
                    }
                } finally {
                    owner.shutdown();
                    long until = System.nanoTime() + TimeUnit.SECONDS.toNanos(11);
                    while (!owner.isStopped() && System.nanoTime() < until) Thread.sleep(10);
                    if (!owner.isStopped()) throw new AssertionError("stale-seam owner exceeded existing shutdown bound");
                }
                if (newChild.isAlive() || Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
                    throw new AssertionError("new exact child or owned publication survived shutdown");
                }
                recording.stop();
                recording.dump(after);
            }
            List<RecordedEvent> events = DiagnosticHandoffProcessScenario.events(after, EVENT);
            if (!expectedStale) {
                if (!events.isEmpty() || !DiagnosticHandoffProcessScenario.delayEvents(after).isEmpty()) {
                    throw new AssertionError("default0/false must not emit handoff or retained-publication events");
                }
                return;
            }
            if (expectedDelayMillis > 0) {
                List<RecordedEvent> delays = DiagnosticHandoffProcessScenario.delayEvents(after);
                if (delays.size() != 1 || events.size() != 1) {
                    throw new AssertionError("startup-configured worker options: real handoff event expected1, got"
                        + delays.size() + "; stale event expected1, got" + events.size());
                }
                RecordedEvent delay = delays.get(0);
                if (delay.getLong("requestedMillis") != expectedDelayMillis
                        || !"COMPLETED".equals(delay.getString("outcome"))
                        || delay.getDuration().compareTo(java.time.Duration.ofMillis(expectedDelayMillis)) < 0
                        || delay.getThread() == null || events.get(0).getThread() == null
                        || delay.getThread().getJavaThreadId() != events.get(0).getThread().getJavaThreadId()) {
                    throw new AssertionError("startup delay must run once on same real stale-release owner worker");
                }
            }
            if (events.size() != 1) {
                throw new AssertionError("revoked authenticated mailbox: real stale release event expected1, got" + events.size());
            }
            RecordedEvent event = events.get(0);
            if (event.getLong("oldEpoch") != oldEpoch || oldEpoch >= renewed
                    || !"AUTHENTICATED".equals(event.getString("state")) || event.getBoolean("oldActive")
                    || event.getBoolean("controlAccepted") || event.getBoolean("bulkAccepted")) {
                throw new AssertionError("real retained old authenticated value must be rejected by both revoked roles");
            }
            if (event.getThread() == null || event.getThread().getJavaThreadId() == Thread.currentThread().getId()
                    || !"nf-local-readonly-owner".equals(event.getThread().getJavaName())) {
                throw new AssertionError("real stale release must run on existing owner worker");
            }
        } catch (IOException unavailable) {
            throw new AssertionError("bounded stale-mailbox JFR capture unavailable");
        }
    }

    private static ProcessHandle exactChild(NodeLaunch launch) {
        List<ProcessHandle> children = ProcessHandle.current().children().filter(candidate ->
            candidate.info().command().map(command -> Path.of(command).equals(launch.executable())).orElse(false))
            .limit(2).toList();
        if (children.size() != 1 || !children.get(0).isAlive()) {
            throw new AssertionError("one live exact authenticated owner child required");
        }
        return children.get(0);
    }
}
