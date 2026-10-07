mod driver_support;
use std::{fs, path::PathBuf};
struct Workspace {
    root: PathBuf,
    parent: PathBuf,
}
impl Workspace {
    fn new() -> Self {
        let workspace =
            fs::canonicalize(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
        let parent = workspace.join(".tmp");
        fs::create_dir_all(&parent).unwrap();
        let parent = fs::canonicalize(parent).unwrap();
        assert!(parent.starts_with(&workspace) && parent != workspace);
        let suffix: String = nf_identity::keys::random_id()
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let root = parent.join(format!("driver-scratch-regression-{suffix}"));
        fs::create_dir(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        assert_eq!(root.parent(), Some(parent.as_path()));
        Self { root, parent }
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        if fs::canonicalize(&self.root).ok().as_ref() == Some(&self.root)
            && self.root.parent() == Some(self.parent.as_path())
            && self.root != self.parent
        {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
#[test]
fn disposable_scratch_creates_its_missing_parent_in_a_fresh_checkout() {
    let workspace = Workspace::new();
    assert!(!workspace.root.join(".tmp").exists());
    let scratch = driver_support::Scratch::new_in(&workspace.root);
    assert!(scratch.0.is_absolute());
    assert!(scratch.0.join("saves").is_dir());
    let owned = scratch.0.clone();
    drop(scratch);
    assert!(!owned.exists());
    assert!(workspace.root.is_dir());
}
#[test]
fn mutable_public_path_cannot_redirect_cleanup_to_an_unrelated_marker() {
    let workspace = Workspace::new();
    fs::create_dir(workspace.root.join(".tmp")).unwrap();
    let mut scratch = driver_support::Scratch::new_in(&workspace.root);
    let owned = scratch.0.clone();
    let unrelated = workspace.root.join("unrelated");
    fs::create_dir(&unrelated).unwrap();
    fs::write(unrelated.join("marker"), b"retain").unwrap();
    scratch.0 = unrelated.clone();
    drop(scratch);
    assert!(
        unrelated.join("marker").exists(),
        "cleanup must use privately captured exact ownership"
    );
    assert!(!owned.exists());
}
