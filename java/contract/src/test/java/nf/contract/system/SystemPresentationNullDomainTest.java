package nf.contract.system;

import java.util.Arrays;
import java.util.HexFormat;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPresentation.Domain;

/** A null domain refuses after a valid namespace; constructor arguments only, data only. */
public final class SystemPresentationNullDomainTest {
    private SystemPresentationNullDomainTest() { }

    private static final String UNIVERSE_HEX = "11111111111111111111111111111111";

    private static void check(boolean condition, String reason) {
        if (!condition) throw new AssertionError(reason);
    }

    private static Object[] argumentImage(byte[] raw, Id namespace, Domain domain) {
        return new Object[] {raw.clone(), namespace.value().clone(), domain};
    }

    private static void fixedArguments(byte[] raw, Id namespace, Domain domain) {
        byte[] expected = HexFormat.of().parseHex(UNIVERSE_HEX);
        check(raw.length == 16 && Arrays.equals(raw, expected), "fixed original nonzero U11 array");
        check(Arrays.equals(namespace.value(), expected), "fixed owned nonzero U11 namespace");
        check(domain == null, "independently fixed null domain");
    }

    private static void nullDomainRefusesAfterValidNamespaceWithoutMutatingArguments() {
        byte[] raw = HexFormat.of().parseHex(UNIVERSE_HEX);
        Id namespace = new Id(raw);
        Domain domain = null;
        fixedArguments(raw, namespace, domain);
        byte[] heldNamespaceBytes = namespace.value();
        byte[] heldNamespaceBefore = heldNamespaceBytes.clone();
        Object[] before = argumentImage(raw, namespace, domain);

        try {
            new SystemPresentation(namespace, domain);
            throw new AssertionError("null domain must refuse after valid namespace");
        } catch (SystemPresentation.Failure failure) {
            check(failure.getClass() == SystemPresentation.Failure.class
                    && failure.code() == SystemPresentation.ErrorCode.BOUNDS,
                    "exact maintained typed bounds failure");
            check(failure.getMessage().equals("BOUNDS") && failure.getCause() == null,
                    "exact public bounds message and null cause");
        }

        fixedArguments(raw, namespace, domain);
        check(Arrays.deepEquals(before, argumentImage(raw, namespace, domain)),
                "constructor refusal preserves complete original argument values");
        check(Arrays.equals(heldNamespaceBytes, heldNamespaceBefore)
                && Arrays.equals(heldNamespaceBytes, HexFormat.of().parseHex(UNIVERSE_HEX)),
                "retained namespace bytes remain independently fixed");
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        nullDomainRefusesAfterValidNamespaceWithoutMutatingArguments();
        System.out.println("Grid256V1 Java null domain after valid namespace PASS; data only");
    }
}
