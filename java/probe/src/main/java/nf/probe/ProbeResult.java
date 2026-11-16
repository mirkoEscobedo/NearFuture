package nf.probe;

/** Closed result codes. No peer text, secret, path, process or throwable enters campaign state. */
public final class ProbeResult {
    public enum Status { REPLIED, UNAVAILABLE, DEADLINE, CANCELLED, REJECTED }
    private final long generation;
    private final byte[] id;
    private final byte[] serviceSession;
    private final byte[] payload;
    private final Status status;

    public ProbeResult(long generation, byte[] id, byte[] serviceSession, Status status, byte[] payload) {
        if (generation <= 0 || id == null || id.length != 16 || serviceSession == null
                || serviceSession.length != 16 || status == null || payload == null
                || (status == Status.REPLIED ? payload.length != 32 : payload.length != 0))
            throw new IllegalArgumentException("Invalid probe result");
        if (status == Status.REPLIED && allZero(serviceSession))
            throw new IllegalArgumentException("Missing service session");
        this.generation = generation;
        this.id = id.clone();
        this.serviceSession = serviceSession.clone();
        this.status = status;
        this.payload = payload.clone();
    }
    private static boolean allZero(byte[] bytes) {
        for (byte value : bytes) if (value != 0) return false;
        return true;
    }
    public long generation() { return generation; }
    public byte[] id() { return id.clone(); }
    public byte[] serviceSession() { return serviceSession.clone(); }
    public Status status() { return status; }
    public byte[] payload() { return payload.clone(); }
    static ProbeResult failure(ProbeRequest request, Status status) {
        return new ProbeResult(request.generation(), request.id(), new byte[16], status, new byte[0]);
    }
}
