use std::{
    fs,
    path::{Path, PathBuf},
};
pub struct Scratch(pub PathBuf, Ownership);
struct Ownership {
    repository: PathBuf,
    workspace: PathBuf,
    parent: PathBuf,
    target: PathBuf,
}
impl Scratch {
    pub fn new() -> Self {
        Self::new_in(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
    }
    #[allow(dead_code)]
    pub fn new_in(workspace: &Path) -> Self {
        let repository =
            fs::canonicalize(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
        let workspace = fs::canonicalize(workspace).unwrap();
        assert!(workspace.starts_with(&repository) && workspace.is_dir());
        let parent = workspace.join(".tmp");
        fs::create_dir_all(&parent).unwrap();
        let parent = fs::canonicalize(parent).unwrap();
        assert!(parent.starts_with(&workspace) && parent != workspace);
        let suffix: String = nf_identity::keys::random_id()
            .unwrap()
            .iter()
            .map(|n| format!("{n:02x}"))
            .collect();
        let target = parent.join(format!("driver-{suffix}"));
        assert!(
            target.is_absolute()
                && target.starts_with(&workspace)
                && target.parent() == Some(parent.as_path())
                && target != parent
        );
        fs::create_dir(&target).unwrap();
        let scratch = Self(
            target.clone(),
            Ownership {
                repository,
                workspace,
                parent,
                target,
            },
        );
        assert_eq!(
            fs::canonicalize(&scratch.1.target).unwrap(),
            scratch.1.target
        );
        fs::create_dir(scratch.1.target.join("saves")).unwrap();
        scratch
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let owned = &self.1;
        // Never use the public path: callers may have changed it after construction.
        // Resolve the privately captured exact target immediately before recursive cleanup.
        if owned.target.is_absolute()
            && owned.target.starts_with(&owned.workspace)
            && owned.workspace.starts_with(&owned.repository)
            && owned.parent.starts_with(&owned.workspace)
            && owned.parent != owned.workspace
            && owned.target.parent() == Some(owned.parent.as_path())
            && owned.target != owned.parent
            && fs::canonicalize(&owned.repository).ok().as_ref() == Some(&owned.repository)
            && fs::canonicalize(&owned.workspace).ok().as_ref() == Some(&owned.workspace)
            && fs::canonicalize(&owned.parent).ok().as_ref() == Some(&owned.parent)
            && fs::canonicalize(&owned.target).ok().as_ref() == Some(&owned.target)
        {
            let _ = fs::remove_dir_all(&owned.target);
        }
    }
}
mod fixture;
#[allow(unused_imports)]
pub use fixture::Fixture;
#[allow(dead_code)]
pub mod process;
