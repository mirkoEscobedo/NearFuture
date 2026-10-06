use std::{
    fs,
    path::{Path, PathBuf},
};
pub struct Disposable(PathBuf);
impl std::ops::Deref for Disposable {
    type Target = PathBuf;
    fn deref(&self) -> &PathBuf {
        &self.0
    }
}
impl AsRef<Path> for Disposable {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}
impl Drop for Disposable {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub fn disposable() -> Disposable {
    let name: String = nf_identity::keys::random_id()
        .unwrap()
        .iter()
        .map(|value| format!("{value:02x}"))
        .collect();
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp");
    fs::create_dir_all(&base).unwrap();
    let path = base.join(format!("identity-{name}"));
    fs::create_dir(&path).unwrap();
    Disposable(path)
}
