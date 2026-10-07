use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
struct OwnedChild(Option<Child>);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
pub trait OwnedOutput {
    fn bounded_output(&mut self) -> Output;
}
impl OwnedOutput for Command {
    /// Test-only ownership: bound this exact child, kill and reap it on every failure path.
    fn bounded_output(&mut self) -> Output {
        self.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut owned = OwnedChild(Some(self.spawn().unwrap()));
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if owned.0.as_mut().unwrap().try_wait().unwrap().is_some() {
                break;
            }
            assert!(Instant::now() < deadline, "owned IPC CLI exceeded deadline");
            std::thread::sleep(Duration::from_millis(10));
        }
        owned.0.take().unwrap().wait_with_output().unwrap()
    }
}
