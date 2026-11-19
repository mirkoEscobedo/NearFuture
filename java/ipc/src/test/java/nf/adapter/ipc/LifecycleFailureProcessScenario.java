package nf.adapter.ipc;

import java.nio.file.Path;
import java.util.concurrent.TimeUnit;
import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Death of each exact owned child, including the final admitted attempt, revokes old status. */
final class LifecycleFailureProcessScenario {
    private LifecycleFailureProcessScenario() { }
    static void run(NodeLaunch launch) throws InterruptedException {
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
