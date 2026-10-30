package nf.adapter.ipc;

import java.nio.file.Path;
import java.io.IOException;
import java.util.List;
import java.util.concurrent.TimeUnit;
import jdk.jfr.Recording;
import jdk.jfr.consumer.RecordedEvent;
import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Death of each exact owned child, including the final admitted attempt, revokes old status. */
final class LifecycleFailureProcessScenario {
    private LifecycleFailureProcessScenario() { }
    static void run(NodeLaunch launch) throws InterruptedException {
        Path capture = launch.vault().getParent().resolve("owner-child-death.jfr");
        long[] pids = new long[3];
        long[] epochs = new long[3];
        try {
            try (Recording recording = new Recording()) {
                recording.enable("nf.OwnerAttempt").withoutStackTrace();
                recording.setMaxSize(4L * 1024 * 1024);
                recording.start();
                runObserved(launch, pids, epochs);
                recording.stop();
                recording.dump(capture);
            }
            List<RecordedEvent> events = DiagnosticHandoffProcessScenario.events(capture, "nf.OwnerAttempt");
            if (events.size() != 3) throw new AssertionError("one real outcome per killed owned child required");
            for (int index = 0; index < 3; index++) {
                RecordedEvent event = events.get(index);
                if (event.getLong("generation") != 201 || event.getInt("attempt") != index + 1
                        || event.getLong("operationEpoch") != epochs[index]
                        || event.getLong("terminalEpoch") != epochs[index] + 1
                        || !"POLL".equals(event.getString("phase"))
                        || !("CHILD_EXIT".equals(event.getString("code")) || "DISCONNECTED".equals(event.getString("code")))
                        || !"NONE".equals(event.getString("detail"))
                        || !event.getBoolean("childCreated") || event.getLong("childPid") != pids[index]
                        || !event.getBoolean("exitObserved") || event.getInt("exitStatus") == 0
                        || !"EXIT_REAPED".equals(event.getString("cleanup"))
                        || !"NONE".equals(event.getString("cleanupFailure"))
                        || ProcessHandle.of(pids[index]).map(ProcessHandle::isAlive).orElse(false)) {
                    throw new AssertionError("killed-child failure, fencing or actual reap facts changed");
                }
                for (int earlier = 0; earlier < index; earlier++) {
                    if (pids[earlier] == pids[index]) throw new AssertionError("successor must be a distinct owned child");
                }
            }
        } catch (IOException unavailable) {
            throw new AssertionError("bounded owner child-death recording unavailable");
        }
    }
    private static void runObserved(NodeLaunch launch, long[] pids, long[] epochs) throws InterruptedException {
        LifecycleOwner owner = new LifecycleOwner(launch);
        LifecycleMailbox mailbox = owner.activate(201);
        try {
            long previous = 0;
            for (int attempt = 0; attempt < 3; attempt++) {
                long authenticated = authenticated(mailbox, previous);
                var children = ProcessHandle.current().children().filter(child -> child.info().command()
                    .map(command -> Path.of(command).equals(launch.executable())).orElse(false)).toList();
                if (children.size() != 1) throw new AssertionError("one exact child required before fault");
                ProcessHandle child = children.get(0);
                pids[attempt] = child.pid();
                epochs[attempt] = authenticated;
                if (!child.destroyForcibly()) throw new AssertionError("owned fault injection failed");
                long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
                while (mailbox.isCurrent(authenticated) && System.nanoTime() < deadline) Thread.sleep(1);
                if (mailbox.isCurrent(authenticated)) throw new AssertionError("failed child left authenticated epoch current");
                previous = authenticated;
            }
            for (int queued = 0; queued < 16; queued++) {
                long epoch = mailbox.currentEpoch();
                var state = mailbox.poll(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK);
                if (state == LifecycleMailbox.State.AUTHENTICATED && mailbox.isCurrent(epoch)) {
                    throw new AssertionError("final child death admitted queued authentication");
                }
            }
        } finally {
            owner.shutdown();
            long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(11);
            while (!owner.isStopped() && System.nanoTime() < deadline) Thread.sleep(10);
            if (!owner.isStopped()) throw new AssertionError("faulted owner shutdown exceeded bound");
        }
    }
    private static long authenticated(LifecycleMailbox mailbox, long after) throws InterruptedException {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(15);
        while (System.nanoTime() < deadline) {
            long epoch = mailbox.currentEpoch();
            var state = mailbox.poll(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL);
            if (state == LifecycleMailbox.State.AUTHENTICATED && epoch > after && mailbox.isCurrent(epoch)) return epoch;
            Thread.sleep(1);
        }
        throw new AssertionError("fault recovery did not authenticate a fresh connection");
    }
}
