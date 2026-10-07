package nf.adapter.ipc;
public final class IpcSmoke {
 private IpcSmoke() { }
 private static void check(boolean value) {if (!value) throw new AssertionError("IPC seam failed");}
 public static void main(String[] arguments) {
  FrameBridge bridge=new FrameBridge(2,12);
  FrameMessage first=new FrameMessage(9,new byte[]{1,2,3,4,5,6});
  FrameMessage second=new FrameMessage(9,new byte[]{7,8,9,10,11,12});
  check(bridge.offer(first));check(bridge.offer(second));check(!bridge.offer(first));
  check(bridge.poll(9)==first);check(bridge.bytes()==6);
  check(bridge.offer(new FrameMessage(8,new byte[]{3})));
  check(bridge.poll(9)==second);check(bridge.poll(9)==null);check(bridge.bytes()==0);
  SessionFence fence=new SessionFence(9);check(fence.accepts(9));fence.invalidate();check(!fence.accepts(9));
  byte[] original={1};FrameMessage immutable=new FrameMessage(9,original);original[0]=3;check(immutable.copyPayload()[0]==1);
  System.out.println("IPC headless handoff/fence passed; no game lifecycle certification.");
 }
}
