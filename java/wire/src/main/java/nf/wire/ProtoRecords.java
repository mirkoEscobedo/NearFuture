package nf.wire;

import com.google.protobuf.Descriptors.EnumValueDescriptor;
import com.google.protobuf.Message;
import java.util.ArrayList;
import java.util.List;
import nf.contract.canonical.Records;
import static nf.wire.SemanticAdmission.*;

/** Explicit mapping from wire messages to the independently specified canonical field order. */
final class ProtoRecords {
    private ProtoRecords() { }
    private static Records.Id id(Message message, String name) { return new Records.Id(bytes(nested(message, name), "value")); }
    private static Records.Digest digest(Message message, String name) { return new Records.Digest(bytes(nested(message, name), "value")); }
    private static Records.U64 counter(Message message, String name) { return new Records.U64(uint64(nested(message, name), "value")); }
    private static Records.Nested record(Message message, String name) { return new Records.Nested(convert(nested(message, name))); }
    private static Records.Documents records(Message message, String name) {
        ArrayList<Records.Document> documents = new ArrayList<>();
        for (Object item : (List<?>) field(message, name)) documents.add(convert((Message) item));
        return new Records.Documents(documents);
    }
    private static Records.Revisions revisions(Message message, String name) {
        ArrayList<Records.Revision> revisions = new ArrayList<>();
        for (Object item : (List<?>) field(message, name)) {
            Message value = (Message) item;
            revisions.add(new Records.Revision(id(value, "aggregate_id"), uint64(nested(value, "revision"), "value")));
        }
        return new Records.Revisions(revisions);
    }
    private static Records.SetValues set(Message message, String name) {
        ArrayList<Long> values = new ArrayList<>();
        for (Object value : (List<?>) field(message, name)) values.add(Integer.toUnsignedLong((Integer) value));
        return new Records.SetValues(values);
    }
    private static Records.Document document(int domain, int kind, Records.Value... fields) { return new Records.Document(domain, kind, List.of(fields)); }
    static Records.Document binding(Message intent) {
        Message principal = nested(intent, "principal");
        return document(1, 1, id(intent, "request_id"), id(principal, "account_id"), id(principal, "device_id"),
                id(intent, "universe_id"), id(intent, "history_id"), new Records.U32(uint32(intent, "operation_kind")), digest(intent, "payload_digest"));
    }
    static Records.Document convert(Message message) {
        String type = message.getDescriptorForType().getName();
        return switch (type) {
            case "SchemaPayload" -> document(6, 1, new Records.U32(uint32(message, "schema_id")), new Records.Bytes(bytes(message, "canonical_body")));
            case "RequiredSemantics" -> document(6, 2, set(message, "capability_ids"), set(message, "schema_ids"));
            case "AggregateVersion" -> document(6, 3, id(message, "aggregate_id"), counter(message, "revision"));
            case "ModuleState" -> document(6, 4, id(message, "provider_id"), new Records.U32(uint32(message, "state_version")), record(message, "state"));
            case "EntityState" -> document(6, 5, id(message, "entity_id"), record(message, "aggregate"), record(message, "state"));
            case "ScheduledEvent" -> document(6, 6, id(message, "schedule_id"), counter(message, "due_tick"), record(message, "event"));
            case "Reservation" -> document(6, 7, id(message, "operation_id"), record(message, "state"));
            case "OperationStatus" -> {
                Records.Document outcome = null;
                if (message.hasField(message.getDescriptorForType().findFieldByName("success"))) outcome = convert(nested(message, "success"));
                else if (message.hasField(message.getDescriptorForType().findFieldByName("error"))) outcome = convert(nested(message, "error"));
                Long committed = message.hasField(message.getDescriptorForType().findFieldByName("committed_event_seq")) ? uint64(nested(message, "committed_event_seq"), "value") : null;
                yield document(6, 8, id(message, "request_id"), id(message, "operation_id"), id(message, "history_id"), digest(message, "request_binding_digest"),
                        new Records.U32(((EnumValueDescriptor) field(message, "phase")).getNumber()), new Records.Outcome(outcome), new Records.OptionalU64(committed));
            }
            case "DeduplicationEntry" -> document(6, 9, id(message, "request_id"), digest(message, "binding_digest"), record(message, "status"));
            case "BoundedError" -> document(6, 10, new Records.U32(((EnumValueDescriptor) field(message, "code")).getNumber()), new Records.Text((String) field(message, "reason")),
                    set(message, "unsupported_capability_ids"), set(message, "unsupported_schema_ids"), new Records.Bool((Boolean) field(message, "retryable")));
            case "Intent" -> {
                Long expiry = message.hasField(message.getDescriptorForType().findFieldByName("expires_at_tick")) ? uint64(message, "expires_at_tick") : null;
                yield document(1, 2, new Records.Nested(binding(message)), revisions(message, "expected_revisions"), record(message, "payload"), new Records.OptionalU64(expiry), record(message, "required"));
            }
            case "WorldSnapshot" -> document(2, 1, id(message, "universe_id"), id(message, "history_id"), counter(message, "event_seq"), counter(message, "world_tick"), digest(message, "ruleset_hash"),
                    revisions(message, "aggregate_revisions"), records(message, "entities"), records(message, "module_state"), records(message, "scheduled_events"), records(message, "reservations"),
                    records(message, "unresolved_operations"), records(message, "deduplication_state"), record(message, "required"));
            case "Proposal" -> document(3, 1, id(message, "job_id"), id(message, "provider_id"), new Records.U32(uint32(message, "provider_version")), digest(message, "ruleset_hash"), digest(message, "input_hash"),
                    counter(message, "runtime_session"), revisions(message, "read_set"), revisions(message, "write_set"), records(message, "proposed_events"), records(message, "module_state_delta"), record(message, "required"));
            case "EventBatch" -> document(4, 1, id(message, "history_id"), counter(message, "event_seq"), digest(message, "previous_hash"), counter(message, "authority_term"), counter(message, "world_tick"), digest(message, "ruleset_hash"),
                    records(message, "request_outcomes"), records(message, "events"), records(message, "module_state_changes"), digest(message, "state_hash"), record(message, "required"));
            default -> throw new WireFailure(WireFailure.Code.UNSUPPORTED);
        };
    }
}
