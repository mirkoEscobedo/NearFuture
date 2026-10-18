package nf.contract.system;

import java.util.Arrays;
import java.util.HexFormat;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPresentation.Domain;

/** A zero namespace refuses before simultaneously invalid numeric bounds; data only. */
public final class SystemPresentationNamespaceTest {
    private SystemPresentationNamespaceTest() { }

    private static final String ZERO_HEX = "00000000000000000000000000000000";

    private static void check(boolean condition, String reason) {
        if (!condition) throw new AssertionError(reason);
    }

    private static Object[] argumentImage(byte[] raw, Id namespace, Domain domain) {
        return new Object[] {raw.clone(), namespace.value().clone(), domain.originX(),
            domain.originY(), domain.width(), domain.height(), domain.halfExtent()};
    }

    private static void fixedArguments(byte[] raw, Id namespace, Domain domain) {
        byte[] zero = HexFormat.of().parseHex(ZERO_HEX);
        check(raw.length == 16 && Arrays.equals(raw, zero), "fixed original zero16 array");
        check(Arrays.equals(namespace.value(), zero), "fixed owned zero16 namespace value");
        check(domain.originX() == 0 && domain.originY() == 0 && domain.width() == 0
                && domain.height() == 0 && domain.halfExtent() == 0, "all five fixed domain fields");
    }

    private static void zeroNamespaceRefusesBeforeNumericBoundsWithoutMutatingArguments() {
        byte[] raw = new byte[16];
        Id namespace = new Id(raw);
        Domain domain = new Domain(0, 0, 0, 0, 0);
        fixedArguments(raw, namespace, domain);
        byte[] heldNamespaceBytes = namespace.value();
        byte[] heldNamespaceBefore = heldNamespaceBytes.clone();
        Object[] before = argumentImage(raw, namespace, domain);

        try {
            new SystemPresentation(namespace, domain);
            throw new AssertionError("zero namespace must refuse before numeric bounds");
        } catch (SystemPresentation.Failure failure) {
            check(failure.code() == SystemPresentation.ErrorCode.INVALID_NAMESPACE,
                    "exact typed namespace refusal before numeric bounds");
            check(failure.getMessage().equals("INVALID_NAMESPACE") && failure.getCause() == null,
                    "exact public namespace refusal message and null cause");
        }

        fixedArguments(raw, namespace, domain);
        check(Arrays.deepEquals(before, argumentImage(raw, namespace, domain)),
                "constructor refusal preserves all original argument bytes and fields");
        check(Arrays.equals(heldNamespaceBytes, heldNamespaceBefore)
                && Arrays.equals(heldNamespaceBytes, HexFormat.of().parseHex(ZERO_HEX)),
                "retained namespace bytes remain fixed");
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        zeroNamespaceRefusesBeforeNumericBoundsWithoutMutatingArguments();
        System.out.println("Grid256V1 Java zero namespace precedes numeric bounds PASS; data only");
    }
}
