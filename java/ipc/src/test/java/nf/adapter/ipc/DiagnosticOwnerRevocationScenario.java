package nf.adapter.ipc;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.concurrent.TimeUnit;
import jdk.jfr.Recording;

/** Revocation during a real one-shot handoff preserves cancellation and factory cleanup facts. */
final class DiagnosticOwnerRevocationScenario {
    private DiagnosticOwnerRevocationScenario() { }

    static void run(NodeLaunch launch) throws InterruptedException {
        Path capture = launch.vault().getParent().resolve("owner-revocation.jfr");
        try {
            ProcessHandle child;
            try (Recording recording = new Recording()) {
                recording.enable("nf.OwnerAttempt").withoutStackTrace();
                recording.enable("nf.DiagnosticHandoffDelay").withoutStackTrace();
                recording.setMaxSize(4L * 1024 * 1024);
                recording.start();
                LifecycleOwner owner = new LifecycleOwner(launch, new DiagnosticFaultPlan(5000));
                try {
                    LifecycleMailbox mailbox = owner.activate(501);
                    child = awaitChild(launch);
                    // Existing cancellation fixtures use the same real100ms observation window.
                    Thread.sleep(100);
                    owner.suspend();
                    if (mailbox.active() || mailbox.currentEpoch() != 0) {
                        throw new AssertionError("revocation must fence before worker cleanup");
                    }
                } finally {
                    owner.shutdown();
                    long until = System.nanoTime() + TimeUnit.SECONDS.toNanos(11);
                    while (!owner.isStopped() && System.nanoTime() < until) Thread.sleep(10);
                    if (!owner.isStopped()) throw new AssertionError("revoked owner exceeded shutdown bound");
                }
                if (child.isAlive()) throw new AssertionError("exact revoked child survived factory cleanup");
                recording.stop();
                recording.dump(capture);
            }
            var events = DiagnosticHandoffProcessScenario.events(capture, "nf.OwnerAttempt");
            if (events.size() != 1) throw new AssertionError("revocation must not launch a successor");
            var event = events.get(0);
            if (event.getLong("generation") != 501 || event.getInt("attempt") != 1
                    || event.getLong("operationEpoch") != 1 || event.getLong("terminalEpoch") != 0
                    || !"HANDOFF_DELAY".equals(event.getString("phase"))
                    || !"CANCELLED".equals(event.getString("code")) || !"NONE".equals(event.getString("detail"))
                    || !event.getBoolean("childCreated") || event.getLong("childPid") != child.pid()
                    || !event.getBoolean("exitObserved") || !"EXIT_REAPED".equals(event.getString("cleanup"))
                    || !"NONE".equals(event.getString("cleanupFailure"))) {
                throw new AssertionError("revoked owner must retain cancellation separately from observed exit");
            }
            var delays = DiagnosticHandoffProcessScenario.delayEvents(capture);
            if (delays.size() != 1 || !"CANCELLED".equals(delays.get(0).getString("outcome"))) {
                throw new AssertionError("cancellation must occur inside the real one-shot handoff");
            }
            if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
                throw new AssertionError("revoked factory left a publication");
            }
        } catch (IOException unavailable) {
            throw new AssertionError("bounded owner revocation recording unavailable");
        }
    }

    private static ProcessHandle awaitChild(NodeLaunch launch) throws InterruptedException {
        long until = System.nanoTime() + TimeUnit.SECONDS.toNanos(15);
        while (System.nanoTime() < until) {
            var children = ProcessHandle.current().children().filter(child -> child.info().command()
                .map(command -> Path.of(command).equals(launch.executable())).orElse(false)).limit(2).toList();
            if (children.size() > 1) throw new AssertionError("only one owned handoff child admitted");
            if (children.size() == 1) return children.get(0);
            Thread.sleep(1);
        }
        throw new AssertionError("real owned handoff child not observed");
    }
}
