package nf.adapter.ipc;

import com.google.protobuf.ByteString;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HexFormat;
import java.util.concurrent.atomic.AtomicBoolean;

/** Owned process integration harness; does not load a game or supply native acceptance. */
public final class ParentNodeProcessClient {
    private ParentNodeProcessClient() { }
    public static void main(String[] args) {
        try { run(args); System.out.println("PARENT_NODE_AUTH_SHUTDOWN_OK"); }
        catch (NodeFailure failure) { System.err.println("PARENT_OWNER_FAILED: " + failure.getMessage()); System.exit(1); }
        catch (Exception failure) { System.err.println("PARENT_OWNER_FAILED: INVALID"); System.exit(1); }
    }
    private static void run(String[] args) throws InterruptedException {
        if (args.length != 5) throw new AssertionError("owned launch arguments");
        ByteString pin = args[4].equals("wrong-pin") ? ByteString.copyFrom(new byte[32])
            : ByteString.copyFrom(HexFormat.of().parseHex(args[3]));
        NodeLaunch launch = new NodeLaunch(Path.of(args[0]), pin,
            Path.of(args[1]), Path.of(args[2]), "java-parent-owner", ParentHandoffFixture.scope(), 60000);
        if (args[4].equals("stalled-child")) { StalledChildProcessScenario.run(launch); return; }
        if (args[4].equals("stalled-handoff-budget")) { DiagnosticStartupBudgetScenario.run(launch); return; }
        if (args[4].equals("abrupt-jvm")) { AbruptJvmProcessScenario.run(launch); return; }
        if (args[4].equals("abrupt-jvm-owner")) { AbruptJvmProcessScenario.runOwner(launch); return; }
        if (args[4].equals("lifecycle")) { LifecycleOwnerProcessScenario.run(launch); return; }
        if (args[4].equals("lifecycle-failure")) { LifecycleFailureProcessScenario.run(launch); return; }
        if (args[4].equals("lifecycle-unlaunchable")) { DiagnosticOwnerFailureScenario.run(launch); return; }
        if (args[4].equals("lifecycle-revocation-outcome")) { DiagnosticOwnerRevocationScenario.run(launch); return; }
        if (args[4].equals("lifecycle-handoff-delay")) { DiagnosticLifecycleHandoffScenario.run(launch); return; }
        if (args[4].equals("lifecycle-stale-mailbox")) { DiagnosticStaleMailboxScenario.run(launch); return; }
        if (args[4].equals("lifecycle-startup-options")) { DiagnosticStartupConfigurationScenario.run(launch); return; }
        if (args[4].equals("lifecycle-default-options") || args[4].equals("lifecycle-zero-options")) {
            DiagnosticDefaultOptionsScenario.run(launch, args[4].equals("lifecycle-zero-options")); return;
        }
        if (args[4].equals("handoff-delay")) { DiagnosticHandoffProcessScenario.run(launch); return; }
        if (args[4].equals("handoff-cancel") || args[4].equals("handoff-interrupt")) {
            DiagnosticHandoffCancellationScenario.run(launch, args[4].equals("handoff-interrupt")); return;
        }
        if (args[4].equals("unlaunchable")) {
            try (OwnedNodeConnection unexpected = OwnedNodeConnection.open(launch, () -> true)) {
                unexpected.runtimeSession();
                throw new AssertionError("invalid executable unexpectedly activated");
            } catch (NodeFailure refused) {
                if (refused.code() != NodeFailure.Code.STARTUP) {
                    throw new AssertionError("unlaunchable executable must fail startup");
                }
                if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))
                        || ProcessHandle.current().children().anyMatch(child -> child.info().command()
                            .map(command -> Path.of(command).equals(launch.executable())).orElse(false))) {
                    throw new AssertionError("unlaunchable startup left publication or owned child");
                }
                if (!"STARTUP CHILD_START_IO".equals(refused.getMessage())
                        || refused.getMessage().length() > 64
                        || refused.getCause() != null || refused.getSuppressed().length != 0) {
                    throw new AssertionError("startup refusal must retain closed CHILD_START_IO");
                }
            }
            return;
        }
        if (args[4].equals("wrong-pin") || args[4].equals("cancelled-before")) {
            NodeFailure.Code expected = args[4].equals("wrong-pin") ? NodeFailure.Code.PIN : NodeFailure.Code.CANCELLED;
            boolean current = args[4].equals("wrong-pin");
            try (OwnedNodeConnection unexpected = OwnedNodeConnection.open(launch, () -> current)) {
                unexpected.runtimeSession();
                throw new AssertionError("unadmitted owner activated");
            } catch (NodeFailure failure) {
                if (failure.code() != expected) throw new AssertionError("wrong owner refusal");
            }
            if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) throw new AssertionError("refused owner published");
            return;
        }
        if (args[4].equals("owner-error")) {
            try (OwnedNodeConnection unexpected = OwnedNodeConnection.open(launch, () -> {
                if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) throw new AssertionError("OWNER_FAULT");
                return true;
            })) {
                unexpected.runtimeSession();
                throw new AssertionError("owner fault was not observed");
            } catch (AssertionError expected) {
                if (!"OWNER_FAULT".equals(expected.getMessage())) throw expected;
            }
            if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
                throw new AssertionError("factory error left an owned publication");
            }
            if (ProcessHandle.current().children().anyMatch(child -> child.info().command()
                    .map(command -> Path.of(command).equals(launch.executable())).orElse(false))) {
                throw new AssertionError("factory error left an owned child");
            }
            return;
        }
        if (!args[4].equals("normal") && !args[4].equals("cancelled-active")) throw new AssertionError("unknown owner scenario");
        AtomicBoolean current = new AtomicBoolean(true);
        long pid;
        try (OwnedNodeConnection connection = OwnedNodeConnection.open(launch, current::get)) {
            if (!connection.authenticatedBoth() || connection.runtimeSession() == 0) {
                throw new AssertionError("both real Rust endpoints must authenticate");
            }
            if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
                throw new AssertionError("authenticated private owner retained discovery publication");
            }
            pid = connection.childPid();
            if (pid <= 0 || !ProcessHandle.of(pid).map(ProcessHandle::isAlive).orElse(false)) {
                throw new AssertionError("owned child must be alive before close");
            }
            if (args[4].equals("cancelled-active")) {
                current.set(false);
                if (connection.authenticatedBoth()) throw new AssertionError("revoked owner remained active");
                try {
                    connection.poll();
                    throw new AssertionError("revoked owner progressed old work");
                } catch (NodeFailure failure) {
                    if (failure.code() != NodeFailure.Code.CANCELLED) throw new AssertionError("revocation refusal");
                }
            }
        }
        if (ProcessHandle.of(pid).map(ProcessHandle::isAlive).orElse(false)) {
            throw new AssertionError("owned child must be reaped after close");
        }
        if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
            throw new AssertionError("owned publication must be removed after normal close");
        }
    }
}
