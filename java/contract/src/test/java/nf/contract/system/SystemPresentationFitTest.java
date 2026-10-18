package nf.contract.system;

import java.util.Arrays;
import java.util.HexFormat;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPresentation.Domain;

/** Bounded numeric domains with insufficient grid pitch refuse; constructor arguments only. */
public final class SystemPresentationFitTest {
    private SystemPresentationFitTest() { }

    private static final String UNIVERSE_HEX = "11111111111111111111111111111111";

    private static final long[][] EXPECTED = {
        {0, 0, 3839, 3840, 7},
        {0, 0, 3840, 3839, 7},
        {0, 0, 255, 256, 0}
    };

    private static void check(boolean condition, String reason) {
        if (!condition) throw new AssertionError(reason);
    }

    private static long[] domainImage(Domain domain) {
        return new long[] {domain.originX(), domain.originY(), domain.width(),
                domain.height(), domain.halfExtent()};
    }

    private static Object[] argumentImage(byte[] raw, Id namespace, Domain domain) {
        return new Object[] {raw.clone(), namespace.value().clone(), domainImage(domain)};
    }

    private static void fixedArguments(byte[] raw, Id namespace, Domain domain, long[] expected) {
        byte[] fixedNamespace = HexFormat.of().parseHex(UNIVERSE_HEX);
        check(raw.length == 16 && Arrays.equals(raw, fixedNamespace), "fixed original nonzero U11");
        check(Arrays.equals(namespace.value(), fixedNamespace), "fixed owned nonzero U11");
        check(Arrays.equals(domainImage(domain), expected), "all five independent fixed domain fields");
    }

    private static void insufficientGridPitchRefusesWithoutMutatingArguments() {
        Domain[] domains = {
            new Domain(0, 0, 3839, 3840, 7),
            new Domain(0, 0, 3840, 3839, 7),
            new Domain(0, 0, 255, 256, 0)
        };
        check(domains.length == 3 && EXPECTED.length == 3, "exact three insufficient-pitch rows");
        int completed = 0;
        for (int index = 0; index < domains.length; index++) {
            byte[] raw = HexFormat.of().parseHex(UNIVERSE_HEX);
            Id namespace = new Id(raw);
            Domain domain = domains[index];
            long[] expected = EXPECTED[index].clone();
            long[] expectedBefore = expected.clone();
            fixedArguments(raw, namespace, domain, expected);
            byte[] heldNamespaceBytes = namespace.value();
            byte[] heldNamespaceBefore = heldNamespaceBytes.clone();
            Object[] before = argumentImage(raw, namespace, domain);

            try {
                new SystemPresentation(namespace, domain);
                throw new AssertionError("insufficient-pitch row returned successfully: " + index);
            } catch (SystemPresentation.Failure failure) {
                check(failure.getClass() == SystemPresentation.Failure.class
                        && failure.code() == SystemPresentation.ErrorCode.DOES_NOT_FIT,
                        "exact maintained typed fit failure at row " + index);
                check(failure.getMessage().equals("DOES_NOT_FIT") && failure.getCause() == null,
                        "exact public fit message and null cause at row " + index);
            }

            fixedArguments(raw, namespace, domain, expected);
            check(Arrays.deepEquals(before, argumentImage(raw, namespace, domain)),
                    "complete original constructor arguments unchanged at row " + index);
            check(Arrays.equals(heldNamespaceBytes, heldNamespaceBefore)
                    && Arrays.equals(heldNamespaceBytes, HexFormat.of().parseHex(UNIVERSE_HEX)),
                    "retained namespace bytes stay independently fixed at row " + index);
            check(Arrays.equals(expected, expectedBefore) && Arrays.equals(expected, EXPECTED[index]),
                    "independent expected row unchanged at row " + index);
            completed++;
        }
        check(completed == 3, "all three actual constructor refusals completed");
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        insufficientGridPitchRefusesWithoutMutatingArguments();
        System.out.println("Grid256V1 Java insufficient grid pitch PASS; data only");
    }
}
