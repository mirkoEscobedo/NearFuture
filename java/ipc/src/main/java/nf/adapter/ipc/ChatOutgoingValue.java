package nf.adapter.ipc;

import org.nearfuture.protocol.v1.ChatOutgoingStatus;

/** Immutable readonly local status projection. It grants no game or remote-signature authority. */
public final class ChatOutgoingValue {
    private final ChatOutgoingStatus value;
    ChatOutgoingValue(ChatOutgoingStatus value) {this.value=value;}
    public ChatOutgoingStatus status() {return value;}
}
