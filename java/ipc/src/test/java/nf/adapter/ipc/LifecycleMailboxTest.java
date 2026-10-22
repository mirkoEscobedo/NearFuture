package nf.adapter.ipc;

import org.nearfuture.ipc.v1.LocalEndpointRole;

/** Independent delayed-value and capacity outcomes through the callback-safe public seam. */
public final class LifecycleMailboxTest {
    private LifecycleMailboxTest() { }
    private static final LocalEndpointRole CONTROL = LocalEndpointRole.LOCAL_ENDPOINT_ROLE_CONTROL;
    private static final LocalEndpointRole BULK = LocalEndpointRole.LOCAL_ENDPOINT_ROLE_BULK;
    private static void check(boolean value, String message) {
        if (!value) throw new AssertionError(message);
    }
    public static void main(String[] args) {
        LifecycleMailbox mailbox = new LifecycleMailbox(17);
        check(mailbox.currentEpoch() == 0, "no connection before admission");
        long first = mailbox.beginConnection();
        check(first > 0, "nonzero connection epoch");
        check(mailbox.publish(CONTROL, first, LifecycleMailbox.State.AUTHENTICATED), "old control queued");
        check(mailbox.publish(BULK, first, LifecycleMailbox.State.AUTHENTICATED), "old bulk queued");
        long second = mailbox.beginConnection();
        check(second > first, "reconnect must revoke the prior connection epoch");
        check(!mailbox.publish(CONTROL, first, LifecycleMailbox.State.AUTHENTICATED), "late producer refused");
        check(mailbox.poll(CONTROL) == null && mailbox.poll(BULK) == null, "delayed old values discarded");
        check(mailbox.publish(CONTROL, second, LifecycleMailbox.State.AUTHENTICATED), "current control queued");
        check(mailbox.poll(CONTROL) == LifecycleMailbox.State.AUTHENTICATED, "current value admitted");
        mailbox.revoke();
        check(!mailbox.active() && mailbox.currentEpoch() == 0 && !mailbox.isCurrent(second), "save/load revocation synchronous");
        check(!mailbox.publish(BULK, second, LifecycleMailbox.State.AUTHENTICATED), "revoked generation cannot publish");
        check(mailbox.poll(CONTROL) == null && mailbox.beginConnection() == 0, "revoked owner cannot revive");
        LifecycleMailbox bounded = new LifecycleMailbox(18);
        long current = bounded.beginConnection();
        for (int item = 0; item < 16; item++) check(bounded.publish(BULK, current, LifecycleMailbox.State.STARTING), "bulk budget");
        check(!bounded.publish(BULK, current, LifecycleMailbox.State.STARTING), "bulk overflow refused");
        check(bounded.publish(CONTROL, current, LifecycleMailbox.State.AUTHENTICATED), "control reservation survives full bulk");
        check(bounded.poll(CONTROL) == LifecycleMailbox.State.AUTHENTICATED, "control progress without draining bulk");
        for (int item = 0; item < 16; item++) check(bounded.poll(BULK) == LifecycleMailbox.State.STARTING, "bounded bulk drain");
        check(bounded.poll(BULK) == null, "empty bulk queue");
        authenticationFailureContextIsClosedAndBounded();
        System.out.println("Lifecycle queue generations and independent role budgets passed; no native lifecycle claim.");
    }

    private static void authenticationFailureContextIsClosedAndBounded() {
        String prefix = "real lifecycle owner did not authenticate both current roles";
        String number = "-?(?:0|[1-9][0-9]*)";
        String state = "(?:NONE|STARTING|AUTHENTICATED|UNAVAILABLE|EXPIRED)";
        String grammar = "\\A" + prefix
            + ";scenario=(?:LIFECYCLE_HANDOFF|RETAINED_STALE|DEFAULT_MISSING|DEFAULT_EXPLICIT_ZERO_FALSE)"
            + ";phase=(?:INITIAL|RENEWAL|REPLACEMENT)"
            + ";afterEpoch=" + number + ";observedEpoch=" + number + ";currentEpoch=" + number
            + ";active=(?:true|false);controlLast=" + state + ";controlSampleEpoch=" + number
            + ";controlAuthSeen=(?:true|false);bulkLast=" + state + ";bulkSampleEpoch=" + number
            + ";bulkAuthSeen=(?:true|false);elapsedMillis=" + number + ";finalSnapshot=NON_ATOMIC\\z";
        String empty = DiagnosticLifecycleHandoffScenario.authFailureContext(
            DiagnosticLifecycleHandoffScenario.AuthCase.DEFAULT_MISSING,
            DiagnosticLifecycleHandoffScenario.Phase.INITIAL, 0, 0, 0, false, null, 0, false, null, 0, false, 0);
        check(empty.equals(prefix + ";scenario=DEFAULT_MISSING;phase=INITIAL;afterEpoch=0;observedEpoch=0"
            + ";currentEpoch=0;active=false;controlLast=NONE;controlSampleEpoch=0;controlAuthSeen=false"
            + ";bulkLast=NONE;bulkSampleEpoch=0;bulkAuthSeen=false;elapsedMillis=0;finalSnapshot=NON_ATOMIC"),
            "empty observations remain NONE and final reads explicitly non-atomic");
        java.util.Set<String> distinctLabels = new java.util.HashSet<>();
        for (DiagnosticLifecycleHandoffScenario.AuthCase scenario : DiagnosticLifecycleHandoffScenario.AuthCase.values()) {
            for (DiagnosticLifecycleHandoffScenario.Phase phase : DiagnosticLifecycleHandoffScenario.Phase.values()) {
                String message = DiagnosticLifecycleHandoffScenario.authFailureContext(scenario, phase,
                    7, 11, 13, true, LifecycleMailbox.State.AUTHENTICATED, 5, true,
                    LifecycleMailbox.State.STARTING, 9, false, 15001);
                check(message.matches(grammar) && message.length() <= 512, "closed bounded diagnostic grammar");
                check(message.contains(";scenario=" + scenario.name() + ";phase=" + phase.name()),
                    "case and lifecycle phase remain distinct");
                check(message.contains(";controlLast=AUTHENTICATED;controlSampleEpoch=5;controlAuthSeen=true")
                    && message.contains(";bulkLast=STARTING;bulkSampleEpoch=9;bulkAuthSeen=false"),
                    "independent last observations and original AUTH flags are not conflated");
                check(distinctLabels.add(message), "each enum case/phase has a distinct label");
            }
        }
        for (long extreme : new long[] {Long.MIN_VALUE, Long.MAX_VALUE}) {
            String message = DiagnosticLifecycleHandoffScenario.authFailureContext(
                DiagnosticLifecycleHandoffScenario.AuthCase.DEFAULT_EXPLICIT_ZERO_FALSE,
                DiagnosticLifecycleHandoffScenario.Phase.REPLACEMENT, extreme, extreme, extreme, false,
                LifecycleMailbox.State.UNAVAILABLE, extreme, false, LifecycleMailbox.State.EXPIRED, extreme, true, extreme);
            check(message.matches(grammar) && message.length() <= 512, "signed-long extrema preserve bound and grammar");
            check(message.contains(extreme == Long.MIN_VALUE ? "-9223372036854775808" : "9223372036854775807"),
                "primitive long extrema are rendered exactly");
            check(!message.contains("/") && !message.contains("\\") && !message.contains("nf.ipc.")
                && !message.contains("\n") && !message.contains("\r"), "no paths, properties or free-text lines");
        }
        try {
            DiagnosticLifecycleHandoffScenario.authFailureContext(null,
                DiagnosticLifecycleHandoffScenario.Phase.INITIAL, 0, 0, 0, false, null, 0, false, null, 0, false, 0);
            throw new AssertionError("missing closed case must be rejected");
        } catch (NullPointerException expected) { }
        try {
            DiagnosticLifecycleHandoffScenario.authFailureContext(
                DiagnosticLifecycleHandoffScenario.AuthCase.DEFAULT_MISSING, null,
                0, 0, 0, false, null, 0, false, null, 0, false, 0);
            throw new AssertionError("missing closed phase must be rejected");
        } catch (NullPointerException expected) { }
    }
}
