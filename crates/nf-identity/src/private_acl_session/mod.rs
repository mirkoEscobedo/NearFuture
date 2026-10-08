mod process;
mod protocol;
use crate::model::IdentityError;
use std::path::Path;

pub(crate) struct Session {
    process: process::Process,
    sequence: u8,
}
impl Session {
    pub fn start() -> Result<Self, IdentityError> {
        Ok(Self {
            process: process::Process::start()?,
            sequence: 0,
        })
    }
    pub fn check(&mut self, paths: &[&Path], initialize: bool) -> Result<(), IdentityError> {
        let sequence = self.sequence.checked_add(1).ok_or(IdentityError::Limit)?;
        let frame = protocol::frame(sequence, initialize, paths)?;
        self.process.request(frame, sequence)?;
        self.sequence = sequence;
        Ok(())
    }
    pub fn finish_readonly(self) -> Result<(), IdentityError> {
        self.process
            .finish(protocol::Completion::ReadOnly, self.sequence)
    }
    pub fn finish_append(self) -> Result<(), IdentityError> {
        self.process
            .finish(protocol::Completion::Append, self.sequence)
    }
}
