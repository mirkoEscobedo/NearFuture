fn required_fields(name: &str) -> &'static [i32] {
    match name.rsplit('.').next().unwrap_or_default() {
        "Sha256Digest" => &[1],
        id if id.ends_with("Id") => &[1],
        "ControlEnvelope" => &[1, 2, 3],
        "LocalAuthEnvelope" => &[1, 2, 3, 4],
        "LocalAuthHello" => &[1, 2, 3, 4, 5],
        "LocalAuthChallenge" => &[1, 2, 3],
        "LocalAuthProof" | "LocalAuthAccepted" => &[1],
        "Handshake" => &[1, 2, 4, 5, 6, 7, 8, 9, 10],
        "NegotiatedSession" => &[1, 2, 3, 4],
        "ProtocolRange" => &[1, 2],
        "ResourceLimits" => &[1, 2, 3, 4, 5, 6, 7, 8],
        "Principal" | "AggregateVersion" | "Reservation" | "CancelOperation" => &[1, 2],
        "SchemaPayload" => &[1, 2],
        "Intent" => &[1, 2, 3, 4, 5, 7, 8, 10],
        "QueryOperation" => &[1, 2, 3, 4],
        "QueryChatOutgoing" => &[1, 2, 3, 4, 5, 6],
        "EnqueueChat" => &[1, 2, 3, 4, 5, 6, 7],
        "ChatEnqueueResult" => &[1],
        "ChatOutgoingStatus" => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        "OperationStatus" => &[1, 2, 3, 4, 5],
        "EntityState" | "ModuleState" | "ScheduledEvent" | "DeduplicationEntry" => &[1, 2, 3],
        "WorldSnapshot" => &[1, 2, 3, 4, 5, 13, 14],
        "Proposal" => &[1, 2, 3, 4, 5, 6, 11],
        "EventBatch" => &[1, 2, 3, 4, 5, 6, 10, 11],
        "BoundedError" => &[1],
        "SnapshotChunk" => &[1, 3, 4, 5, 6],
        _ => &[],
    }
}
use crate::{Limits, WireError};
use prost::Message;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{DescriptorProto, FieldDescriptorProto, FileDescriptorSet};
use std::collections::BTreeMap;
use std::sync::OnceLock;

fn registry() -> &'static BTreeMap<String, DescriptorProto> {
    static REGISTRY: OnceLock<BTreeMap<String, DescriptorProto>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let set = FileDescriptorSet::decode(
            include_bytes!(concat!(env!("OUT_DIR"), "/near_future_descriptor.bin")).as_slice(),
        )
        .expect("trusted generated descriptors must decode");
        let mut messages = BTreeMap::new();
        for file in set.file {
            let package = file.package.unwrap_or_default();
            for message in file.message_type {
                messages.insert(
                    format!(".{package}.{}", message.name.as_deref().unwrap_or_default()),
                    message,
                );
            }
        }
        messages
    })
}

struct Scanner {
    limits: Limits,
    entries: usize,
    decoded_bytes: usize,
}
impl Scanner {
    fn charge(&mut self, bytes: usize) -> Result<(), WireError> {
        self.decoded_bytes = self
            .decoded_bytes
            .checked_add(bytes)
            .ok_or(WireError::Limit)?;
        if self.decoded_bytes > self.limits.decoded_bytes {
            return Err(WireError::Limit);
        }
        Ok(())
    }
    fn entry(&mut self) -> Result<(), WireError> {
        self.entries = self.entries.checked_add(1).ok_or(WireError::Limit)?;
        if self.entries > self.limits.total_entries {
            return Err(WireError::Limit);
        }
        Ok(())
    }
    fn message(&mut self, input: &[u8], name: &str, depth: usize) -> Result<(), WireError> {
        if depth > self.limits.depth {
            return Err(WireError::Limit);
        }
        let descriptor = registry().get(name).ok_or(WireError::Unsupported)?;
        let mut seen = vec![0u8; descriptor.field.len()];
        let mut counts = vec![0usize; descriptor.field.len()];
        let mut oneofs = vec![false; descriptor.oneof_decl.len()];
        let mut cursor = Cursor { input, position: 0 };
        while cursor.position < input.len() {
            self.entry()?;
            let tag = cursor.varint()?;
            let number = tag >> 3;
            let wire = (tag & 7) as u8;
            if number == 0 || number > 0x1fff_ffff || matches!(wire, 3 | 4 | 6 | 7) {
                return Err(WireError::Malformed);
            }
            let Some(index) = descriptor
                .field
                .iter()
                .position(|f| f.number == Some(number as i32))
            else {
                if name != ".nearfuture.protocol.v1.TransportMetadata" {
                    return Err(WireError::UnknownField);
                }
                self.skip(&mut cursor, wire)?;
                continue;
            };
            let field = &descriptor.field[index];
            let repeated = field.label() == Label::Repeated;
            let packable = repeated
                && matches!(
                    field.r#type(),
                    Type::Uint32 | Type::Enum | Type::Bool | Type::Fixed64
                );
            let packed = packable && wire == 2;
            let mode = if packed { 2 } else { 1 };
            if seen[index] != 0 && (!repeated || seen[index] != mode || packed) {
                return Err(WireError::Duplicate);
            }
            if !repeated && let Some(group) = field.oneof_index {
                if oneofs[group as usize] {
                    return Err(WireError::Duplicate);
                }
                oneofs[group as usize] = true;
            }
            seen[index] = mode;
            if packed {
                let length = cursor.length()?;
                if length > self.limits.field_bytes {
                    return Err(WireError::Limit);
                }
                let bytes = cursor.take(length)?;
                let mut packed_cursor = Cursor {
                    input: bytes,
                    position: 0,
                };
                while packed_cursor.position < bytes.len() {
                    self.entry()?;
                    counts[index] += 1;
                    self.check_count(name, counts[index])?;
                    self.scalar(
                        &mut packed_cursor,
                        field.r#type(),
                        if field.r#type() == Type::Fixed64 {
                            1
                        } else {
                            0
                        },
                    )?;
                }
            } else {
                counts[index] += 1;
                if repeated {
                    self.check_count(name, counts[index])?;
                }
                self.value(&mut cursor, wire, field, name, depth)?;
            }
        }
        for number in required_fields(name) {
            let index = descriptor
                .field
                .iter()
                .position(|f| f.number == Some(*number))
                .ok_or(WireError::Semantic)?;
            if seen[index] == 0 {
                return Err(WireError::Semantic);
            }
        }
        for (index, declaration) in descriptor.oneof_decl.iter().enumerate() {
            // proto3 optional scalar fields have synthetic oneofs beginning with '_'.
            if !declaration
                .name
                .as_deref()
                .unwrap_or_default()
                .starts_with('_')
                && !oneofs[index]
            {
                // OperationStatus outcome is conditional on its phase.
                if !name.ends_with(".OperationStatus") {
                    return Err(WireError::Semantic);
                }
            }
        }
        Ok(())
    }
    fn check_count(&self, message: &str, count: usize) -> Result<(), WireError> {
        let cap = if matches!(
            message.rsplit('.').next(),
            Some("RequiredSemantics" | "Handshake" | "BoundedError")
        ) {
            64.min(self.limits.collection_items)
        } else {
            self.limits.collection_items
        };
        if count > cap {
            return Err(WireError::Limit);
        }
        Ok(())
    }
    fn value(
        &mut self,
        cursor: &mut Cursor<'_>,
        wire: u8,
        field: &FieldDescriptorProto,
        message: &str,
        depth: usize,
    ) -> Result<(), WireError> {
        match field.r#type() {
            Type::Message => {
                if wire != 2 {
                    return Err(WireError::Malformed);
                }
                let length = cursor.length()?;
                if length > self.limits.frame_bytes {
                    return Err(WireError::Limit);
                }
                self.charge(256)?;
                self.message(
                    cursor.take(length)?,
                    field.type_name.as_deref().ok_or(WireError::Malformed)?,
                    depth + 1,
                )
            }
            Type::Bytes | Type::String => {
                if wire != 2 {
                    return Err(WireError::Malformed);
                }
                let length = cursor.length()?;
                let cap = if field.r#type() == Type::String {
                    if message.ends_with(".BoundedError") {
                        512.min(self.limits.text_bytes)
                    } else if message.ends_with(".EnqueueChat")
                        && field.name.as_deref() == Some("text")
                    {
                        2048.min(self.limits.text_bytes)
                    } else {
                        self.limits.text_bytes
                    }
                } else if message
                    .rsplit('.')
                    .next()
                    .is_some_and(|name| name.starts_with("LocalAuth"))
                {
                    32.min(self.limits.field_bytes)
                } else if message.ends_with(".ChatOutgoingStatus")
                    && field.name.as_deref() == Some("signed_message")
                {
                    2221.min(self.limits.field_bytes)
                } else if message.ends_with(".ChatOutgoingStatus")
                    && field.name.as_deref() == Some("receiver_receipt")
                {
                    323.min(self.limits.field_bytes)
                } else if field.name.as_deref() == Some("signature") || message.ends_with(".PeerId")
                {
                    128.min(self.limits.field_bytes)
                } else {
                    self.limits.field_bytes
                };
                if length > cap {
                    return Err(WireError::Limit);
                }
                self.charge(length)?;
                let bytes = cursor.take(length)?;
                if field.r#type() == Type::Bytes {
                    if field.name.as_deref() == Some("signature") && length != 0 && length != 64 {
                        return Err(WireError::Semantic);
                    }
                    let kind = message.rsplit('.').next().unwrap_or_default();
                    if (kind.ends_with("Id") && kind != "PeerId" && length != 16)
                        || (kind == "Sha256Digest" && length != 32)
                        || (kind.starts_with("LocalAuth") && length != 32)
                        || (kind == "Handshake"
                            && field.name.as_deref() == Some("session_token")
                            && length != 32)
                    {
                        return Err(WireError::Semantic);
                    }
                }
                if field.r#type() == Type::String {
                    let value = std::str::from_utf8(bytes).map_err(|_| WireError::Malformed)?;
                    if !nf_contract::canonical::text::is_canonical_text(value) {
                        return Err(WireError::Semantic);
                    }
                }
                Ok(())
            }
            scalar => self.scalar(cursor, scalar, wire),
        }
    }
    fn scalar(&mut self, cursor: &mut Cursor<'_>, kind: Type, wire: u8) -> Result<(), WireError> {
        self.charge(8)?;
        match (kind, wire) {
            (Type::Fixed64, 1) => {
                cursor.take(8)?;
            }
            (Type::Uint32 | Type::Enum | Type::Bool, 0) => {
                let value = cursor.varint()?;
                if value > u32::MAX as u64
                    || (kind == Type::Bool && value > 1)
                    || (kind == Type::Enum && value > i32::MAX as u64)
                {
                    return Err(WireError::Malformed);
                }
            }
            _ => return Err(WireError::Malformed),
        }
        Ok(())
    }
    fn skip(&mut self, cursor: &mut Cursor<'_>, wire: u8) -> Result<(), WireError> {
        match wire {
            0 => {
                cursor.varint()?;
                self.charge(8)?;
            }
            1 => {
                cursor.take(8)?;
                self.charge(8)?;
            }
            2 => {
                let length = cursor.length()?;
                if length > self.limits.field_bytes {
                    return Err(WireError::Limit);
                }
                cursor.take(length)?;
                self.charge(length)?;
            }
            5 => {
                cursor.take(4)?;
                self.charge(4)?;
            }
            _ => return Err(WireError::Malformed),
        }
        Ok(())
    }
}
struct Cursor<'a> {
    input: &'a [u8],
    position: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], WireError> {
        let end = self.position.checked_add(count).ok_or(WireError::Limit)?;
        let bytes = self
            .input
            .get(self.position..end)
            .ok_or(WireError::Malformed)?;
        self.position = end;
        Ok(bytes)
    }
    fn varint(&mut self) -> Result<u64, WireError> {
        let mut value = 0u64;
        for index in 0..10 {
            let byte = self.take(1)?[0];
            if index == 9 && byte > 1 {
                return Err(WireError::Malformed);
            }
            value |= ((byte & 127) as u64) << (index * 7);
            if byte & 128 == 0 {
                if index > 0 && byte == 0 {
                    return Err(WireError::Malformed);
                }
                return Ok(value);
            }
        }
        Err(WireError::Malformed)
    }
    fn length(&mut self) -> Result<usize, WireError> {
        usize::try_from(self.varint()?).map_err(|_| WireError::Limit)
    }
}
pub(crate) fn preflight(input: &[u8], message: &str, limits: Limits) -> Result<(), WireError> {
    let qualified = format!(".nearfuture.protocol.v1.{message}");
    preflight_qualified(input, &qualified, limits)
}
pub(crate) fn preflight_qualified(
    input: &[u8],
    message: &str,
    limits: Limits,
) -> Result<(), WireError> {
    limits.validate()?;
    if input.len() > limits.frame_bytes {
        return Err(WireError::Limit);
    }
    let mut scanner = Scanner {
        limits,
        entries: 0,
        decoded_bytes: 0,
    };
    scanner.message(input, message, 1)
}
