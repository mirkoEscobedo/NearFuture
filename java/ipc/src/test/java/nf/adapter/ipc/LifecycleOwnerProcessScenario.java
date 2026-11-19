package nf.adapter.ipc;

import java.nio.file.Files;
import java.util.concurrent.TimeUnit;
import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Actual background owner outcomes, independent of any game callback or worker internals. */
final class LifecycleOwnerProcessScenario {
    private LifecycleOwnerProcessScenario() { }
    static void run(NodeLaunch launch) throws InterruptedException {
        LifecycleOwner owner = new LifecycleOwner(launch);
        try {
            LifecycleMailbox first = owner.activate(101);
            long epoch = authenticated(first, 0);
            long until = System.nanoTime() + TimeUnit.SECONDS.toNanos(6);
            while (System.nanoTime() < until) {
                if (!first.active()) throw new AssertionError("live generation revoked unexpectedly");
                Thread.sleep(10);
            }
            long renewed = authenticated(first, epoch);
            if (renewed <= epoch) throw new AssertionError("connection did not renew before idle expiry");
            LifecycleMailbox second = owner.activate(102);
            if (first.active() || first.currentEpoch() != 0) throw new AssertionError("old campaign remained current");
            authenticated(second, 0);
            owner.suspend();
            if (second.active() || second.currentEpoch() != 0) throw new AssertionError("save did not revoke synchronously");
        } finally {
            owner.shutdown();
            long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(11);
            while (!owner.isStopped() && System.nanoTime() < deadline) Thread.sleep(10);
            if (!owner.isStopped()) throw new AssertionError("background owner exceeded shutdown bound");
        }
        if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) throw new AssertionError("actor left publication");
        if (ProcessHandle.current().children().anyMatch(child -> child.info().command()
                .map(command -> java.nio.file.Path.of(command).equals(launch.executable())).orElse(false))) {
            throw new AssertionError("actor left owned child");
        }
    }
    private static long authenticated(LifecycleMailbox mailbox, long after) throws InterruptedException {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(15);
        while (System.nanoTime() < deadline) {
            long epoch = mailbox.currentEpoch();
            var state = mailbox.poll(LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL);
            if (state == LifecycleMailbox.State.AUTHENTICATED && epoch > after && mailbox.isCurrent(epoch)) return epoch;
            Thread.sleep(10);
        }
        throw new AssertionError("background owner did not publish a fresh authenticated connection");
    }
}
