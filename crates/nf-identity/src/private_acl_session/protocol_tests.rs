use super::{Completion, completion, frame};
use crate::model::IdentityError;
use std::{
    ffi::OsString,
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
};

#[test]
fn canonical_frames_and_only_complete_read_or_append_terminals_are_admitted() {
    let path = Path::new("C:/public/entry");
    assert_eq!(
        frame(1, false, &[path]),
        Ok(b"V 1 1\nC:/public/entry\n".to_vec())
    );
    assert_eq!(
        frame(4, true, &[path]),
        Ok(b"I 4 1\nC:/public/entry\n".to_vec())
    );
    assert_eq!(completion(Completion::ReadOnly, 2), Ok(b"R 2\n".to_vec()));
    assert_eq!(completion(Completion::Append, 6), Ok(b"A 6\n".to_vec()));
    for partial in [0, 1, 3, 4, 5, 7] {
        assert_eq!(
            completion(Completion::ReadOnly, partial),
            Err(IdentityError::PrivateStorage)
        );
        assert_eq!(
            completion(Completion::Append, partial),
            Err(IdentityError::PrivateStorage)
        );
    }
    assert_eq!(
        completion(Completion::Append, 2),
        Err(IdentityError::PrivateStorage)
    );
    assert_eq!(
        completion(Completion::ReadOnly, 6),
        Err(IdentityError::PrivateStorage)
    );
    assert_eq!(frame(4, false, &[path]), Err(IdentityError::PrivateStorage));
    assert_eq!(frame(1, true, &[path]), Err(IdentityError::PrivateStorage));
    assert_eq!(
        frame(2, false, &[path, Path::new("C:/public/other")]),
        Err(IdentityError::PrivateStorage)
    );
}

#[test]
fn complete_maximum_inventory_and_utf8_byte_boundary_remain_bounded() {
    let maximum: Vec<PathBuf> = (0..65)
        .map(|index| PathBuf::from(format!("{index:02}{}", "p".repeat(4094))))
        .collect();
    let paths: Vec<&Path> = maximum.iter().map(PathBuf::as_path).collect();
    let encoded = frame(1, false, &paths).unwrap();
    assert!(encoded.starts_with(b"V 1 65\n"));
    assert_eq!(encoded.len(), 266312);
    assert_eq!(encoded.iter().filter(|byte| **byte == b'\n').count(), 66);
    assert!(encoded.len() < 272384);
    let mut too_many = paths.clone();
    too_many.push(Path::new("last"));
    assert_eq!(frame(1, false, &too_many), Err(IdentityError::Limit));
    let unicode_limit = PathBuf::from("λ".repeat(2048));
    let unicode_over = PathBuf::from("λ".repeat(2049));
    assert_eq!(frame(1, false, &[&unicode_limit]).unwrap().len(), 4103);
    assert_eq!(frame(1, false, &[&unicode_over]), Err(IdentityError::Limit));
}

#[test]
fn unframed_and_colliding_paths_cannot_be_admitted() {
    assert_eq!(frame(1, false, &[]), Err(IdentityError::Limit));
    for text in ["", "public\npath", "public\rpath", "public\0path"] {
        assert_eq!(
            frame(1, false, &[Path::new(text)]),
            Err(IdentityError::Limit)
        );
    }
    assert_eq!(
        frame(
            1,
            false,
            &[Path::new("C:/public/Entry"), Path::new("c:/PUBLIC/entry")]
        ),
        Err(IdentityError::PrivateStorage)
    );
    let invalid = PathBuf::from(OsString::from_wide(&[0xd800]));
    assert_eq!(
        frame(1, false, &[&invalid]),
        Err(IdentityError::PrivateStorage)
    );
    assert_eq!(
        frame(0, false, &[Path::new("valid")]),
        Err(IdentityError::Limit)
    );
    assert_eq!(
        frame(7, false, &[Path::new("valid")]),
        Err(IdentityError::Limit)
    );
}
