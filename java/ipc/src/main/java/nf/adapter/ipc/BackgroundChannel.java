package nf.adapter.ipc;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.channels.SocketChannel;
import java.util.Arrays;
/** Background-only nonblocking transport. It owns no game object, thread, file or process. */
public final class BackgroundChannel implements AutoCloseable {
 private final SocketChannel channel;
 private final int maximum;
 private final ByteBuffer header=ByteBuffer.allocate(4);
 private ByteBuffer inbound,outbound;
 public BackgroundChannel(InetSocketAddress address,int maximum) throws IOException {
  if (address.isUnresolved() || !address.getAddress().isLoopbackAddress() || maximum<1 || maximum>1_048_576) throw new IllegalArgumentException("Invalid local IPC endpoint");
  this.maximum=maximum;channel=SocketChannel.open();
  try {channel.configureBlocking(false);channel.connect(address);} catch (IOException | RuntimeException failure) {channel.close();throw failure;}
 }
 public boolean send(byte[] value) {
  if (value==null || value.length==0 || value.length>maximum) throw new IllegalArgumentException("Invalid frame length");
  if (outbound!=null) return false;
  outbound=ByteBuffer.allocate(value.length+4);outbound.putInt(value.length).put(value).flip();return true;
 }
 /** Performs at most one 4096-byte write and one 4096-byte read. Never waits for readiness. */
 public byte[] poll() throws IOException {
  if (!channel.isConnected() && !channel.finishConnect()) return null;
  if (outbound!=null) {
   limitedWrite(outbound);
   if (!outbound.hasRemaining()) {erase(outbound);outbound=null;}
  }
  ByteBuffer target=inbound==null?header:inbound;
  int result=limitedRead(target);if (result<0) throw new IOException("IPC disconnected");
  if (target.hasRemaining()) return null;
  if (inbound==null) {
   header.flip();long length=Integer.toUnsignedLong(header.getInt());header.clear();
   if (length==0 || length>maximum) throw new IOException("Invalid bounded IPC frame");
   inbound=ByteBuffer.allocate((int)length);return null;
  }
  byte[] completed=inbound.array();inbound=null;return completed;
 }
 private int limitedRead(ByteBuffer value) throws IOException {int limit=value.limit();value.limit(Math.min(limit,value.position()+4096));try {return channel.read(value);} finally {value.limit(limit);}}
 private void limitedWrite(ByteBuffer value) throws IOException {int limit=value.limit();value.limit(Math.min(limit,value.position()+4096));try {channel.write(value);} finally {value.limit(limit);}}
 private static void erase(ByteBuffer value) {if (value!=null) Arrays.fill(value.array(),(byte)0);}
 @Override public void close() throws IOException {erase(inbound);erase(outbound);Arrays.fill(header.array(),(byte)0);channel.close();}
}
