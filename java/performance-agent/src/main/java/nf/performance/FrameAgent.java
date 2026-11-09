package nf.performance;

import java.lang.instrument.Instrumentation;
import java.lang.instrument.ClassFileTransformer;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.security.ProtectionDomain;
import java.util.HexFormat;
import java.util.concurrent.atomic.AtomicBoolean;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassVisitor;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.MethodVisitor;
import org.objectweb.asm.Opcodes;

/** Explicit startup-only diagnostic. Never attaches or retransforms a live process. */
public final class FrameAgent {
    private FrameAgent() { }
    public static void premain(String arguments, Instrumentation instrumentation) {
        String trace = System.getenv("NF_PERFORMANCE_TRACE_ID");
        String observer = System.getenv("NF_PERFORMANCE_OBSERVER");
        if (arguments == null || !arguments.matches("[0-9a-f]{64}")
            || trace == null || !trace.matches("[0-9a-f]{32}")
            || (observer != null && !observer.equals("true") && !observer.equals("false"))) {
            reject(); return;
        }
        String configuredTrace = System.getProperty("nf.profiling.traceId");
        String configuredObserver = System.getProperty("nf.profiling.callback.enabled");
        if ((configuredTrace != null && !configuredTrace.equals(trace))
            || (observer != null && configuredObserver != null && !configuredObserver.equals(observer))) {
            reject(); return;
        }
        // Application-startup configuration only. Per-run anonymous correlation
        // and explicit observer choice do not rewrite the fixed launcher artifact.
        System.setProperty("nf.profiling.traceId", trace);
        if (observer != null) System.setProperty("nf.profiling.callback.enabled", observer);
        FrameProbe.configure(trace, arguments, "true".equals(System.getProperty("nf.profiling.callback.enabled")));
        instrumentation.addTransformer(new PresentationTransformer(HexFormat.of().parseHex(arguments)), false);
    }
    private static void reject() {
        FrameProbe.reject(1);
        System.err.println("NF_FRAME_PROBE_REJECTED");
    }
    private static final class PresentationTransformer implements ClassFileTransformer {
        private final byte[] expected;
        private final AtomicBoolean claimed = new AtomicBoolean();
        PresentationTransformer(byte[] expected) { this.expected = expected.clone(); }

        @Override public byte[] transform(ClassLoader loader, String name, Class<?> redefining,
                ProtectionDomain domain, byte[] bytes) {
            if (!"org/lwjgl/opengl/Display".equals(name)) return null;
            try {
                if (redefining != null || bytes == null || bytes.length > 1_048_576
                    || !MessageDigest.isEqual(expected, MessageDigest.getInstance("SHA-256").digest(bytes))
                    || !claimed.compareAndSet(false, true)) {
                    reject(); return null;
                }
                ClassReader reader = new ClassReader(bytes);
                ClassWriter writer = new ClassWriter(0);
                int[] matched = {0};
                reader.accept(new ClassVisitor(Opcodes.ASM9, writer) {
                    @Override public MethodVisitor visitMethod(int access, String method, String descriptor,
                            String signature, String[] exceptions) {
                        MethodVisitor original = super.visitMethod(access, method, descriptor, signature, exceptions);
                        if (!method.equals("swapBuffers") || !descriptor.equals("()V")) return original;
                        int required = Opcodes.ACC_PUBLIC | Opcodes.ACC_STATIC;
                        if ((access & required) != required || (access & (Opcodes.ACC_NATIVE | Opcodes.ACC_ABSTRACT)) != 0) {
                            throw new IllegalArgumentException("Unsupported presentation shape");
                        }
                        matched[0]++;
                        return new MethodVisitor(Opcodes.ASM9, original) {
                            @Override public void visitInsn(int instruction) {
                                if (instruction == Opcodes.RETURN) {
                                    super.visitMethodInsn(Opcodes.INVOKESTATIC, "nf/performance/FrameProbe", "present", "()V", false);
                                }
                                super.visitInsn(instruction);
                            }
                        };
                    }
                }, 0);
                if (matched[0] != 1) { reject(); return null; }
                return writer.toByteArray();
            } catch (RuntimeException | NoSuchAlgorithmException unsupported) {
                reject(); return null;
            }
        }
    }
}
