package nf.adapter.ipc;

import java.io.IOException;
import java.nio.file.Files;
import java.util.HexFormat;
import java.util.concurrent.TimeUnit;

/** Kills only a directly owned JVM after real two-role authentication, bypassing its close hooks. */
final class AbruptJvmProcessScenario {
    private AbruptJvmProcessScenario() { }
    static void runOwner(NodeLaunch launch) throws InterruptedException {
        try (OwnedNodeConnection connection = OwnedNodeConnection.open(launch, () -> true)) {
            if (!connection.authenticatedBoth()) throw new AssertionError("abrupt owner authentication");
            System.out.println("OWNED_CHILD " + connection.childPid());
            System.out.flush();
            Thread.sleep(60000);
            throw new AssertionError("abrupt owner was not terminated");
        }
    }
    static void run(NodeLaunch launch) throws InterruptedException {
        Process parent = null;
        ProcessHandle child = null;
        try {
            String java = ProcessHandle.current().info().command().orElseThrow();
            parent = new ProcessBuilder(java, "-cp", System.getProperty("java.class.path"),
                ParentNodeProcessClient.class.getName(), launch.executable().toString(),
                launch.vault().toString(), launch.saves().toString(),
                HexFormat.of().formatHex(launch.executableSha256().toByteArray()), "abrupt-jvm-owner")
                .redirectError(ProcessBuilder.Redirect.DISCARD).start();
            String line = readyLine(parent);
            if (!line.matches("OWNED_CHILD [1-9][0-9]{0,18}")) throw new AssertionError("abrupt owner signal");
            long pid = Long.parseLong(line.substring(12));
            child = parent.children().filter(handle -> handle.pid() == pid).findFirst().orElseThrow();
            if (!child.isAlive()) throw new AssertionError("authenticated child already dead");
            if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
                throw new AssertionError("authenticated discovery must already be retired");
            }
            parent.destroyForcibly();
            if (!parent.waitFor(5, TimeUnit.SECONDS)) throw new AssertionError("owned JVM not reaped");
            long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
            while (child.isAlive() && System.nanoTime() - deadline < 0) Thread.sleep(10);
            if (child.isAlive()) throw new AssertionError("Rust child survived abrupt JVM pipe closure");
            if (Files.exists(launch.vault().resolve("blob-java-parent-owner"))) {
                throw new AssertionError("abrupt death left discovery");
            }
        } catch (IOException failure) {
            throw new AssertionError("abrupt JVM scenario IO", failure);
        } finally {
            if (parent != null) {
                if (parent.isAlive()) parent.destroyForcibly();
                if (!parent.waitFor(2, TimeUnit.SECONDS)) throw new AssertionError("owned JVM cleanup");
            }
            if (child != null && child.isAlive()) {
                child.destroyForcibly();
                long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2);
                while (child.isAlive() && System.nanoTime() - deadline < 0) Thread.sleep(10);
                if (child.isAlive()) throw new AssertionError("owned Rust cleanup");
            }
        }
    }
    private static String readyLine(Process parent) throws IOException, InterruptedException {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(15);
        StringBuilder line = new StringBuilder();
        while (System.nanoTime() - deadline < 0) {
            if (!parent.isAlive()) throw new AssertionError("JVM exited before authenticated signal");
            if (parent.getInputStream().available() == 0) { Thread.sleep(1); continue; }
            int next = parent.getInputStream().read();
            if (next == '\n') return line.toString().replace("\r", "");
            if (next < 0 || next > 127 || line.length() >= 64) throw new AssertionError("bounded owner signal");
            line.append((char)next);
        }
        throw new AssertionError("owned JVM authentication deadline");
    }
}
