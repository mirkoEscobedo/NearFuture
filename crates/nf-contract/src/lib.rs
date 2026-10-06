#![no_std]

extern crate alloc;

pub mod canonical;

// Data-only build identity shared by the initial synthetic node.
// Wire schemas and game lifecycle contracts are separate roadmap work.

/// The synthetic build cannot certify a Starsector integration.
pub const BUILD_IDENTITY: &str = "Near Future 0.1.0 (synthetic; game integration unavailable)";

pub mod identity;
pub mod signatures;
