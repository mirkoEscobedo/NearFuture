//! Thin, trusted built-in adapters. Domain libraries retain their own contracts.
//! No ambient storage, networking, game reference or social identity is injected.
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

pub const API_VERSION: u16 = 1;
pub const PROBE_MODULE: u16 = 1;
pub const PROBE_CAPABILITY: u32 = 1;
pub const MAX_PAYLOAD: usize = 65_408;
pub const MAX_REPLY: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Error {
    Malformed = 1,
    UnsupportedVersion = 2,
    UnsupportedCapability = 3,
    Limit = 4,
    UnsupportedModule = 5,
    UnsupportedOperation = 6,
    Disabled = 7,
    StaleBinding = 8,
    Deadline = 9,
    Cancelled = 10,
    Unavailable = 11,
    InvalidSnapshot = 12,
}

pub struct Context<'a> {
    pub deadline: Instant,
    pub cancelled: &'a AtomicBool,
}
impl Context<'_> {
    pub fn check(&self) -> Result<(), Error> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        if Instant::now() >= self.deadline {
            return Err(Error::Deadline);
        }
        Ok(())
    }
}

pub struct Invocation<'a> {
    pub session: [u8; 16],
    pub request_id: [u8; 16],
    /// Zero is service-neutral. A nonzero value must match the current owner binding.
    pub campaign_generation: u64,
    pub module: u16,
    pub operation: u16,
    pub api_version: u16,
    pub required_capabilities: u32,
    pub payload: &'a [u8],
}

#[derive(Debug, Eq, PartialEq)]
pub struct Reply {
    pub session: [u8; 16],
    pub request_id: [u8; 16],
    pub campaign_generation: u64,
    pub outcome: Result<Vec<u8>, Error>,
}

pub trait Module {
    fn invoke(&self, payload: &[u8], context: &Context<'_>) -> Result<Vec<u8>, Error>;
}

/// A real application operation: echo exactly one caller-supplied 32-byte challenge.
pub struct Probe;
impl Module for Probe {
    fn invoke(&self, payload: &[u8], context: &Context<'_>) -> Result<Vec<u8>, Error> {
        context.check()?;
        if payload.len() != 32 {
            return Err(Error::Malformed);
        }
        Ok(payload.to_vec())
    }
}

/// Explicit composition, not an arbitrary plugin registry or security sandbox.
pub struct Host {
    session: [u8; 16],
    generation: u64,
    probe: Option<Probe>,
    active: bool,
}
impl Host {
    pub fn new(session: [u8; 16], probe: bool) -> Result<Self, Error> {
        if session == [0; 16] {
            return Err(Error::Malformed);
        }
        Ok(Self {
            session,
            generation: 0,
            probe: probe.then_some(Probe),
            active: true,
        })
    }
    pub fn session(&self) -> [u8; 16] {
        self.session
    }
    pub fn campaign_generation(&self) -> u64 {
        self.generation
    }
    pub fn bind_campaign(&mut self, generation: u64) -> Result<(), Error> {
        if !self.active || generation <= self.generation {
            return Err(Error::StaleBinding);
        }
        self.generation = generation;
        Ok(())
    }
    pub fn stop(&mut self) {
        self.active = false;
    }
    pub fn available(&self) -> u32 {
        if self.active && self.probe.is_some() {
            PROBE_CAPABILITY
        } else {
            0
        }
    }
    pub fn dispatch(&self, request: &Invocation<'_>, granted: u32, context: &Context<'_>) -> Reply {
        let outcome = self.execute(request, granted, context).and_then(|value| {
            context.check()?;
            if value.len() > MAX_REPLY {
                return Err(Error::Limit);
            }
            Ok(value)
        });
        Reply {
            session: self.session,
            request_id: request.request_id,
            campaign_generation: request.campaign_generation,
            outcome,
        }
    }
    fn execute(
        &self,
        r: &Invocation<'_>,
        granted: u32,
        context: &Context<'_>,
    ) -> Result<Vec<u8>, Error> {
        context.check()?;
        if !self.active {
            return Err(Error::Unavailable);
        }
        if r.session != self.session
            || (r.campaign_generation != 0 && r.campaign_generation != self.generation)
        {
            return Err(Error::StaleBinding);
        }
        if r.request_id == [0; 16] {
            return Err(Error::Malformed);
        }
        if r.payload.len() > MAX_PAYLOAD {
            return Err(Error::Limit);
        }
        if r.api_version != API_VERSION {
            return Err(Error::UnsupportedVersion);
        }
        if r.required_capabilities & !granted != 0 {
            return Err(Error::UnsupportedCapability);
        }
        let capability = match r.module {
            PROBE_MODULE => PROBE_CAPABILITY,
            _ => return Err(Error::UnsupportedModule),
        };
        if r.operation != 1 {
            return Err(Error::UnsupportedOperation);
        }
        if self.available() & capability == 0 {
            return Err(Error::Disabled);
        }
        if granted & capability == 0 {
            return Err(Error::UnsupportedCapability);
        }
        match r.module {
            PROBE_MODULE => self
                .probe
                .as_ref()
                .ok_or(Error::Disabled)?
                .invoke(r.payload, context),
            _ => Err(Error::UnsupportedModule),
        }
    }
}
