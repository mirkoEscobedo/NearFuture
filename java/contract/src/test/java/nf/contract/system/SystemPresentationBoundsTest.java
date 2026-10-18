package nf.contract.system;

import java.util.Arrays;
import java.util.HexFormat;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPresentation.Domain;

/** Numeric admission refuses invalid domains before fit; constructor arguments only, data only. */
public final class SystemPresentationBoundsTest {
    private SystemPresentationBoundsTest() { }

    private static final String UNIVERSE_HEX = "11111111111111111111111111111111";

    private static final long[][] EXPECTED = {
        {-1099511627777L, -2048, 8192, 16384, 7},
        {1099511627777L, -2048, 8192, 16384, 7},
        {-1024, -1099511627777L, 8192, 16384, 7},
        {-1024, 1099511627777L, 8192, 16384, 7},
        {-1024, -2048, 0, 16384, 7},
        {-1024, -2048, 16777217, 16384, 7},
        {-1024, -2048, 8192, 0, 7},
        {-1024, -2048, 8192, 16777217, 7},
        {-1024, -2048, 8192, 16384, 32768},
        {-1024, -2048, -1, 16384, 7},
        {-1024, -2048, 8192, -1, 7},
        {-1024, -2048, 8192, 16384, -1}
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

    private static void invalidNumericDomainsRefuseBeforeFitWithoutMutatingArguments() {
        Domain[] domains = {
            new Domain(-1099511627777L, -2048, 8192, 16384, 7),
            new Domain(1099511627777L, -2048, 8192, 16384, 7),
            new Domain(-1024, -1099511627777L, 8192, 16384, 7),
            new Domain(-1024, 1099511627777L, 8192, 16384, 7),
            new Domain(-1024, -2048, 0, 16384, 7),
            new Domain(-1024, -2048, 16777217, 16384, 7),
            new Domain(-1024, -2048, 8192, 0, 7),
            new Domain(-1024, -2048, 8192, 16777217, 7),
            new Domain(-1024, -2048, 8192, 16384, 32768),
            new Domain(-1024, -2048, -1, 16384, 7),
            new Domain(-1024, -2048, 8192, -1, 7),
            new Domain(-1024, -2048, 8192, 16384, -1)
        };
        check(domains.length == 12 && EXPECTED.length == 12, "exact twelve numeric partitions");
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
                throw new AssertionError("invalid numeric row returned successfully: " + index);
            } catch (SystemPresentation.Failure failure) {
                check(failure.getClass() == SystemPresentation.Failure.class
                        && failure.code() == SystemPresentation.ErrorCode.BOUNDS,
                        "exact maintained typed bounds failure at row " + index);
                check(failure.getMessage().equals("BOUNDS") && failure.getCause() == null,
                        "exact public bounds message and null cause at row " + index);
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
        check(completed == 12, "all twelve actual constructor refusals completed");
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        invalidNumericDomainsRefuseBeforeFitWithoutMutatingArguments();
        System.out.println("Grid256V1 Java invalid numeric domains precede fit PASS; data only");
    }
}
