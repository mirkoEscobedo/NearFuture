package nf.wire;

import com.google.protobuf.ByteString;
import com.google.protobuf.Descriptors.FieldDescriptor;
import com.google.protobuf.Descriptors.EnumValueDescriptor;
import com.google.protobuf.Message;
import java.nio.charset.StandardCharsets;
import java.text.Normalizer;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import nf.contract.canonical.Canonical;
import nf.contract.canonical.RecordCodec;
import nf.contract.canonical.Records;
import nf.contract.canonical.Validation;

/** Validates presence and registered meaning before constructing canonical domain data. */
final class SemanticAdmission {
    private SemanticAdmission() { }
    static Object field(Message message, String name) { return message.getField(message.getDescriptorForType().findFieldByName(name)); }
    static Message nested(Message message, String name) { return (Message) field(message, name); }
    static byte[] bytes(Message message, String name) { return ((ByteString) field(message, name)).toByteArray(); }
    static long uint32(Message message, String name) { return Integer.toUnsignedLong((Integer) field(message, name)); }
    static long uint64(Message message, String name) { return (Long) field(message, name); }
    static void require(Message message, String... names) {
        for (String name : names) if (!message.hasField(message.getDescriptorForType().findFieldByName(name))) throw new WireFailure(WireFailure.Code.SEMANTIC);
    }
    static void validate(Message message) {
        String name = message.getDescriptorForType().getName();
        if (!name.equals("TransportMetadata") && !message.getUnknownFields().asMap().isEmpty()) throw new WireFailure(WireFailure.Code.UNKNOWN_FIELD);
        for (var entry : message.getAllFields().entrySet()) {
            FieldDescriptor descriptor = entry.getKey();
            if (descriptor.getJavaType() == FieldDescriptor.JavaType.MESSAGE) {
                if (descriptor.isRepeated() && ((List<?>) entry.getValue()).size() > 4096) throw new WireFailure(WireFailure.Code.LIMIT);
                if (descriptor.isRepeated()) for (Object item : (List<?>) entry.getValue()) validate((Message) item);
                else validate((Message) entry.getValue());
            } else if (descriptor.getJavaType() == FieldDescriptor.JavaType.BYTE_STRING && ((ByteString) entry.getValue()).size() > 262_144) {
                throw new WireFailure(WireFailure.Code.LIMIT);
            } else if (descriptor.getJavaType() == FieldDescriptor.JavaType.STRING) {
                String text = (String) entry.getValue();
                if (text.length() > (name.equals("BoundedError") ? 512 : 4096)) throw new WireFailure(WireFailure.Code.LIMIT);
                try { Canonical.validateText(text, name.equals("BoundedError") ? 512 : 4096); }
                catch (Validation rejected) { throw new WireFailure(rejected.code() == Validation.Code.LIMIT_EXCEEDED ? WireFailure.Code.LIMIT : WireFailure.Code.SEMANTIC); }
                if (text.getBytes(StandardCharsets.UTF_8).length > (name.equals("BoundedError") ? 512 : 4096)) throw new WireFailure(WireFailure.Code.LIMIT);
            }
        }
        if (name.endsWith("Id") && !name.equals("PeerId")) width(bytes(message, "value"), 16);
        if (name.equals("Sha256Digest")) width(bytes(message, "value"), 32);
        switch (name) {
            case "ControlEnvelope" -> {
                require(message, "runtime_session", "required");
                if (uint32(message, "protocol_version") != 1) throw new WireFailure(WireFailure.Code.UNSUPPORTED);
                Message body = null;
                for (FieldDescriptor descriptor : message.getDescriptorForType().getOneofs().get(0).getFields()) if (message.hasField(descriptor)) body = (Message) message.getField(descriptor);
                if (body == null) throw new WireFailure(WireFailure.Code.SEMANTIC);
                String bodyName = body.getDescriptorForType().getName();
                if (bodyName.equals("CancelOperation")) throw new WireFailure(WireFailure.Code.UNSUPPORTED);
                Message bodyRequired = null;
                if (List.of("Handshake", "Intent", "Proposal", "EventBatch").contains(bodyName)) bodyRequired = nested(body, "required");
                if (bodyName.equals("HandshakeResult") && body.hasField(body.getDescriptorForType().findFieldByName("accepted"))) {
                    Message accepted = nested(body, "accepted");
                    bodyRequired = nested(accepted, "semantics");
                    if (uint64(nested(message, "runtime_session"), "value") != uint64(nested(accepted, "runtime_session"), "value")) throw new WireFailure(WireFailure.Code.SEMANTIC);
                }
                if (bodyRequired != null && !nested(message, "required").equals(bodyRequired)) throw new WireFailure(WireFailure.Code.SEMANTIC);
                if (List.of("Handshake", "Proposal").contains(bodyName) && uint64(nested(message, "runtime_session"), "value") != uint64(nested(body, "runtime_session"), "value")) throw new WireFailure(WireFailure.Code.SEMANTIC);
            }
            case "RequiredSemantics" -> {
                for (String field : List.of("capability_ids", "schema_ids")) for (long id : sorted(message, field)) if (id != 1) throw new WireFailure(WireFailure.Code.UNSUPPORTED);
            }
            case "Principal" -> require(message, "account_id", "device_id");
            case "Handshake" -> {
                require(message, "protocols", "required", "universe_id", "history_id", "runtime_session", "ruleset_hash", "content_policy_hash", "limits");
                width(bytes(message, "session_token"), 32);
                sorted(message, "optional_capability_ids");
            }
            case "ProtocolRange" -> {
                long minimum = uint32(message, "minimum"), maximum = uint32(message, "maximum");
                if (minimum == 0 || maximum < minimum) throw new WireFailure(WireFailure.Code.SEMANTIC);
                if (minimum > 1 || maximum < 1) throw new WireFailure(WireFailure.Code.UNSUPPORTED);
            }
            case "ResourceLimits" -> limits(message);
            case "NegotiatedSession" -> { require(message, "semantics", "limits", "runtime_session"); if (uint32(message, "protocol_version") != 1) throw new WireFailure(WireFailure.Code.UNSUPPORTED); }
            case "HandshakeResult" -> {
                if (!message.hasField(message.getDescriptorForType().findFieldByName("accepted")) && !message.hasField(message.getDescriptorForType().findFieldByName("rejected"))) throw new WireFailure(WireFailure.Code.SEMANTIC);
            }
            case "Intent" -> {
                require(message, "request_id", "principal", "universe_id", "history_id", "payload", "payload_digest", "required");
                int signatureBytes = ((ByteString) field(message, "signature")).size();
                if (signatureBytes != 0 && signatureBytes != 64) throw new WireFailure(WireFailure.Code.SEMANTIC);
                if (uint32(message, "operation_kind") != 1) throw new WireFailure(WireFailure.Code.UNSUPPORTED);
                canonical(message);
            }
            case "QueryOperation" -> require(message, "request_id", "principal", "universe_id", "history_id");
            case "CancelOperation" -> throw new WireFailure(WireFailure.Code.UNSUPPORTED);
            case "AggregateVersion" -> require(message, "aggregate_id", "revision");
            case "SchemaPayload" -> {
                if (uint32(message, "schema_id") != 1) throw new WireFailure(WireFailure.Code.UNSUPPORTED);
                try { Canonical.decodeProfile(bytes(message, "canonical_body")); }
                catch (Validation malformed) { throw new WireFailure(WireFailure.Code.SEMANTIC); }
            }
            case "OperationStatus" -> {
                require(message, "request_id", "operation_id", "history_id", "request_binding_digest");
                canonical(message);
            }
            case "EntityState" -> require(message, "entity_id", "aggregate", "state");
            case "ModuleState" -> { require(message, "provider_id", "state"); if (uint32(message, "state_version") != 1) throw new WireFailure(WireFailure.Code.UNSUPPORTED); }
            case "ScheduledEvent" -> require(message, "schedule_id", "due_tick", "event");
            case "Reservation" -> require(message, "operation_id", "state");
            case "DeduplicationEntry" -> { require(message, "request_id", "binding_digest", "status"); canonical(message); }
            case "WorldSnapshot" -> {
                require(message, "universe_id", "history_id", "event_seq", "world_tick", "ruleset_hash", "state_hash", "required");
                byte[] actual = Canonical.sha256(canonical(message));
                if (!Arrays.equals(actual, bytes(nested(message, "state_hash"), "value"))) throw new WireFailure(WireFailure.Code.SEMANTIC);
            }
            case "Proposal" -> {
                require(message, "job_id", "provider_id", "ruleset_hash", "input_hash", "runtime_session", "required");
                if (uint32(message, "provider_version") != 1) throw new WireFailure(WireFailure.Code.UNSUPPORTED);
                canonical(message);
            }
            case "EventBatch" -> { require(message, "history_id", "event_seq", "previous_hash", "authority_term", "world_tick", "ruleset_hash", "state_hash", "required"); canonical(message); }
            case "BoundedError" -> {
                long code = ((EnumValueDescriptor) field(message, "code")).getNumber();
                if (code < 1 || code > 10) throw new WireFailure(WireFailure.Code.SEMANTIC);
                sorted(message, "unsupported_capability_ids"); sorted(message, "unsupported_schema_ids");
            }
            case "SnapshotChunk" -> {
                require(message, "transfer_id", "snapshot_digest");
                long count = uint32(message, "chunk_count"), index = uint32(message, "chunk_index"), total = uint64(message, "total_bytes");
                if (count < 1 || count > 256 || index >= count || total == 0 || Long.compareUnsigned(total, 67_108_864) > 0 || ((ByteString) field(message, "data")).size() > 262_144) throw new WireFailure(WireFailure.Code.LIMIT);
                int dataBytes = ((ByteString) field(message, "data")).size();
                if (dataBytes == 0 || dataBytes > total || total > count * 262_144L) throw new WireFailure(WireFailure.Code.SEMANTIC);
            }
            default -> { }
        }
    }
    private static void width(byte[] bytes, int width) { if (bytes.length != width) throw new WireFailure(WireFailure.Code.SEMANTIC); }
    private static List<Long> sorted(Message message, String name) {
        FieldDescriptor descriptor = message.getDescriptorForType().findFieldByName(name);
        int count = message.getRepeatedFieldCount(descriptor);
        if (count > 64) throw new WireFailure(WireFailure.Code.LIMIT);
        ArrayList<Long> values = new ArrayList<>(count);
        long previous = -1;
        for (int i = 0; i < count; i++) {
            long value = Integer.toUnsignedLong((Integer) message.getRepeatedField(descriptor, i));
            if (value == 0 || value <= previous) throw new WireFailure(WireFailure.Code.SEMANTIC);
            values.add(value); previous = value;
        }
        return values;
    }
    private static void limits(Message message) {
        String[] names = {"control_frame_bytes", "chunk_bytes", "inflight_bytes", "inflight_items", "decoded_bytes", "collection_items", "nesting_depth"};
        long[] caps = {1_048_576, 262_144, 16_777_216, 256, 1_048_576, 4096, 32};
        for (int i = 0; i < names.length; i++) { long value = uint32(message, names[i]); if (value < 1 || value > caps[i]) throw new WireFailure(WireFailure.Code.LIMIT); }
        long transfer = uint64(message, "transfer_bytes");
        if (transfer == 0 || Long.compareUnsigned(transfer, 67_108_864) > 0) throw new WireFailure(WireFailure.Code.LIMIT);
        if (Long.compareUnsigned(uint32(message, "chunk_bytes"), transfer) > 0 || uint32(message, "chunk_bytes") > uint32(message, "control_frame_bytes") || uint32(message, "control_frame_bytes") > uint32(message, "decoded_bytes")
                || uint32(message, "control_frame_bytes") > uint32(message, "inflight_bytes")) throw new WireFailure(WireFailure.Code.SEMANTIC);
    }
    static byte[] canonical(Message message) {
        try { return RecordCodec.encode(toRecord(message)); }
        catch (Validation rejected) { throw new WireFailure(rejected.code() == Validation.Code.UNKNOWN_REQUIRED_SEMANTIC ? WireFailure.Code.UNSUPPORTED : WireFailure.Code.SEMANTIC); }
    }
    static Records.Document toRecord(Message message) {
        return ProtoRecords.convert(message);
    }
}
