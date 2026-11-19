// Benign test executable: no sockets, publication, files, stdout or game inputs.
// Deliberately ignores the owner's pipe closure so forced cleanup is observable.
fn main() {
    loop {
        std::thread::park();
    }
}
