use crate::model::IdentityError;
use std::path::Path;

pub(super) const INPUT_LIMIT: usize = 272384;
pub(super) const OUTPUT_LIMIT: usize = 64;
pub(super) fn frame(
    sequence: u8,
    initialize: bool,
    paths: &[&Path],
) -> Result<Vec<u8>, IdentityError> {
    if !(1..=6).contains(&sequence)
        || paths.is_empty()
        || paths.len() > 65
        || (initialize && paths.len() != 1)
    {
        return Err(IdentityError::Limit);
    }
    if initialize != (sequence == 4) || (matches!(sequence, 2 | 3 | 4 | 6) && paths.len() != 1) {
        return Err(IdentityError::PrivateStorage);
    }
    let mut input = Vec::with_capacity(INPUT_LIMIT);
    input.extend_from_slice(
        format!(
            "{} {sequence} {}\n",
            if initialize { "I" } else { "V" },
            paths.len()
        )
        .as_bytes(),
    );
    for (index, path) in paths.iter().enumerate() {
        let text = path.to_str().ok_or(IdentityError::PrivateStorage)?;
        if text.is_empty()
            || text.len() > 4096
            || text.bytes().any(|v| matches!(v, 0 | b'\r' | b'\n'))
            || input.len() + text.len() + 1 > INPUT_LIMIT
        {
            return Err(IdentityError::Limit);
        }
        if paths[..index].iter().any(|previous| {
            previous
                .to_str()
                .is_some_and(|previous| previous.eq_ignore_ascii_case(text))
        }) {
            return Err(IdentityError::PrivateStorage);
        }
        input.extend_from_slice(text.as_bytes());
        input.push(b'\n');
    }
    Ok(input)
}

pub(super) enum Completion {
    ReadOnly,
    Append,
}
pub(super) fn completion(mode: Completion, sequence: u8) -> Result<Vec<u8>, IdentityError> {
    match (mode, sequence) {
        (Completion::ReadOnly, 2) => Ok(b"R 2\n".to_vec()),
        (Completion::Append, 6) => Ok(b"A 6\n".to_vec()),
        _ => Err(IdentityError::PrivateStorage),
    }
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
