package nf.contract.system;

import java.util.Arrays;
import nf.contract.system.SystemPackageTopology.Id;
import nf.contract.system.SystemPresentation.Domain;

/** A null namespace refuses before a null domain; constructor arguments only, data only. */
public final class SystemPresentationNullNamespaceTest {
    private SystemPresentationNullNamespaceTest() { }

    private static void check(boolean condition, String reason) {
        if (!condition) throw new AssertionError(reason);
    }

    private static void nullNamespaceRefusesBeforeNullDomainWithoutMutatingArguments() {
        Id namespace = null;
        Domain domain = null;
        Object[] expected = {null, null};
        Object[] before = {namespace, domain};
        check(namespace == null && domain == null && Arrays.deepEquals(before, expected),
                "both original constructor arguments are independently fixed null");

        try {
            new SystemPresentation(namespace, domain);
            throw new AssertionError("null namespace must refuse before null domain bounds");
        } catch (SystemPresentation.Failure failure) {
            check(failure.getClass() == SystemPresentation.Failure.class
                    && failure.code() == SystemPresentation.ErrorCode.INVALID_NAMESPACE,
                    "exact maintained typed invalid-namespace failure");
            check(failure.getMessage().equals("INVALID_NAMESPACE") && failure.getCause() == null,
                    "exact public refusal message and null cause");
        }

        Object[] after = {namespace, domain};
        check(namespace == null && domain == null, "both original arguments remain null");
        check(Arrays.deepEquals(before, after) && Arrays.deepEquals(after, expected),
                "constructor refusal preserves complete owned null argument images");
    }

    public static void main(String[] args) {
        check(args.length == 0, "no arguments");
        nullNamespaceRefusesBeforeNullDomainWithoutMutatingArguments();
        System.out.println("Grid256V1 Java null namespace precedes null domain PASS; data only");
    }
}
