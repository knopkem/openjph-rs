//! OpenJPH-RS: Pure Rust HTJ2K (JPEG 2000 Part 15) codec
//!
//! A 1:1 port of the OpenJPH C++ library (v0.26.3).

pub mod types;
pub mod error;
pub mod message;
pub mod arch;
pub mod mem;
pub mod file;
pub mod arg;
pub mod params;
pub mod codestream;
pub mod coding;
pub mod transform;

pub use types::*;
pub use error::{OjphError, Result};
