//! `rvp` — the Revised Store package format (`.rvp`): pack, unpack, hash, verify.
#![forbid(unsafe_code)]

pub mod config;
pub mod container;
pub mod deps;
pub mod error;
pub mod hash;
pub mod manifest;
pub mod pack;
pub mod smsvc;
pub mod stock;
pub mod trust;
pub mod ustar;
pub mod verify;

pub use error::RvpError;
