package nf.adapter.ipc;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReferenceArray;
/** Single producer/single consumer immutable handoff. One nonblocking attempt; no I/O or waits.
 * Owner must assign one producer and one consumer and fence before applying a polled value. */
public final class FrameBridge {
 private final AtomicReferenceArray<FrameMessage> slots;
 private final AtomicLong head=new AtomicLong(),tail=new AtomicLong(),bytes=new AtomicLong();
 private final int mask;
 private final long byteLimit;
 public FrameBridge(int capacity,long byteLimit) {
  if (capacity<1 || capacity>256 || (capacity&(capacity-1))!=0 || byteLimit<1 || byteLimit>16_777_216) throw new IllegalArgumentException("Invalid bridge budget");
  slots=new AtomicReferenceArray<>(capacity);mask=capacity-1;this.byteLimit=byteLimit;
 }
 public boolean offer(FrameMessage value) {
  if (value==null) throw new IllegalArgumentException("Missing handoff");
  long position=head.get();
  if (position-tail.get()>=slots.length() || bytes.get()+value.size()>byteLimit) return false;
  bytes.addAndGet(value.size());slots.set((int)(position&mask),value);head.lazySet(position+1);return true;
 }
 /** Removes at most one item. A stale item is discarded; callers can retry next frame. */
 public FrameMessage poll(long session) {
  long position=tail.get();if (position==head.get()) return null;
  int index=(int)(position&mask);FrameMessage value=slots.get(index);
  if (value==null) return null;
  slots.set(index,null);bytes.addAndGet(-value.size());tail.lazySet(position+1);
  return value.session()==session?value:null;
 }
 public long bytes() {return bytes.get();}
}
