//! `rvp` — the Revised Store package format (`.rvp`): pack, unpack, hash, verify.
#![forbid(unsafe_code)]

pub mod container;
pub mod deps;
pub mod error;
pub mod hash;
pub mod manifest;
pub mod pack;
pub mod stock;
pub mod ustar;
pub mod verify;

pub use error::RvpError;
