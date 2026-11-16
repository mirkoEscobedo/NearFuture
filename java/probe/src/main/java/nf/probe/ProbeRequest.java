package nf.probe;

import java.util.Arrays;

/** Immutable bounded values; never contains a game reference or authentication secret. */
public final class ProbeRequest {
    private final long generation;
    private final byte[] id;
    private final byte[] challenge;
    private final long deadline;

    ProbeRequest(long generation, byte[] id, byte[] challenge, long deadline) {
        if (generation <= 0 || id == null || id.length != 16 || challenge == null || challenge.length != 32)
            throw new IllegalArgumentException("Invalid probe values");
        boolean present = false;
        for (byte value : id) present |= value != 0;
        if (!present) throw new IllegalArgumentException("Missing probe request ID");
        this.generation = generation;
        this.id = id.clone();
        this.challenge = challenge.clone();
        this.deadline = deadline;
    }
    public long generation() { return generation; }
    public byte[] id() { return id.clone(); }
    public byte[] challenge() { return challenge.clone(); }
    public long deadlineNanos() { return deadline; }
    public boolean expired() { return System.nanoTime() - deadline >= 0; }
    ProbeRequest boundedTo(int millis) {
        long now = System.nanoTime();
        long remaining = Math.min(deadline - now, millis * 1_000_000L);
        return new ProbeRequest(generation, id, challenge, now + remaining);
    }
    boolean matches(ProbeResult result) {
        return generation == result.generation() && Arrays.equals(id, result.id());
    }
}
