package nf.adapter.ipc;
/** A lifecycle hook must invalidate before load/save/exit, including before stale apply. */
public final class SessionFence {
 private volatile long current;
 public SessionFence(long session) {if (session==0) throw new IllegalArgumentException("Missing session");current=session;}
 public void invalidate() {current=0;}
 public boolean accepts(long session) {return session!=0 && current==session;}
}
