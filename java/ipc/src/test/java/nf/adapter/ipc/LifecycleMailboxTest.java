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
        System.out.println("Lifecycle queue generations and independent role budgets passed; no native lifecycle claim.");
    }
}
