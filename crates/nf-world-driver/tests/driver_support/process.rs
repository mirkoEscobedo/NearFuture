use std::{
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};
struct OwnedChild(Option<Child>);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
/// Own only this exact test child; native verification additionally owns its descendants.
pub fn run(mut command: Command) -> Output {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut owned = OwnedChild(Some(
        command.spawn().expect("owned foreground driver child"),
    ));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let child = owned.0.as_mut().unwrap();
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                assert!(Instant::now() < deadline, "owned driver child timeout");
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => panic!("owned driver child wait failure"),
        }
    }
    owned
        .0
        .take()
        .unwrap()
        .wait_with_output()
        .expect("owned child output")
}
