package nf.adapter.ipc;
/** Small immutable prepared data only. Create/encode outside the campaign advance callback. */
public final class FrameMessage {
 private final long session;
 private final byte[] payload;
 public FrameMessage(long session, byte[] payload) {
  if (session==0 || payload==null || payload.length==0 || payload.length>8192) throw new IllegalArgumentException("Invalid bounded handoff");
  this.session=session;this.payload=payload.clone();
 }
 public long session() {return session;}
 public int size() {return payload.length;}
 public byte[] copyPayload() {return payload.clone();}
}
