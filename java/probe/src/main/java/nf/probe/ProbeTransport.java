package nf.probe;

import java.io.IOException;
import java.util.function.BooleanSupplier;

/** Worker-only port. Implementations authenticate before sending application values, enforce
 * the request deadline/cancellation and validate session, ID, generation and echo before returning. */
public interface ProbeTransport extends AutoCloseable {
    ProbeResult exchange(ProbeRequest request, BooleanSupplier cancelled) throws IOException;
    @Override void close() throws IOException;

    @FunctionalInterface
    interface Factory extends AutoCloseable {
        ProbeTransport open(ProbeRequest request, BooleanSupplier cancelled) throws IOException;
        @Override default void close() throws IOException { }
    }
}
