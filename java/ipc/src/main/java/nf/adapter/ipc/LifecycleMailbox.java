package nf.adapter.ipc;

import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Value-only control/bulk handoff. A callback may revoke and poll; it performs no I/O or waits. */
public final class LifecycleMailbox {
    public enum State { STARTING, AUTHENTICATED, UNAVAILABLE, EXPIRED }
    private final long generation;
    private final SessionFence fence;
    private final FrameBridge control = new FrameBridge(16, 131072);
    private final FrameBridge bulk = new FrameBridge(16, 131072);
    private volatile long epoch;
    private long nextEpoch;

    public LifecycleMailbox(long generation) {
        if (generation <= 0) throw new NodeFailure(NodeFailure.Code.INVALID);
        this.generation = generation;
        this.fence = new SessionFence(generation);
    }
    public boolean active() { return fence.accepts(generation); }
    long generation() { return generation; }
    public long currentEpoch() { return active() ? epoch : 0; }
    /** Called only by the single background owner; revocation may occur on the callback thread. */
    public long beginConnection() {
        if (!active()) return 0;
        if (nextEpoch == Long.MAX_VALUE) { revoke(); return 0; }
        long fresh = ++nextEpoch;
        epoch = fresh;
        return isCurrent(fresh) ? fresh : 0;
    }
    public boolean isCurrent(long candidate) { return candidate != 0 && active() && epoch == candidate; }
    public boolean publish(LocalEndpointRole role, long candidate, State state) {
        if (state == null) throw new NodeFailure(NodeFailure.Code.INVALID);
        if (!isCurrent(candidate)) return false;
        return bridge(role).offer(new FrameMessage(candidate, new byte[]{(byte)state.ordinal()}));
    }
    /** Removes at most one value; caller must recheck isCurrent before using a returned value. */
    public State poll(LocalEndpointRole role) {
        long expected = currentEpoch();
        if (expected == 0) return null;
        FrameMessage message = bridge(role).poll(expected);
        if (message == null || !isCurrent(expected)) return null;
        byte[] value = message.copyPayload();
        if (value.length != 1 || Byte.toUnsignedInt(value[0]) >= State.values().length) return null;
        return State.values()[Byte.toUnsignedInt(value[0])];
    }
    public void revoke() { fence.invalidate(); epoch = 0; }
    private FrameBridge bridge(LocalEndpointRole role) {
        if (role == LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL) return control;
        if (role == LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK) return bulk;
        throw new NodeFailure(NodeFailure.Code.INVALID);
    }
}
