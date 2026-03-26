//! Prelude module providing commonly used imports
//!
//! Copyright (c) 2025 Erwan Patrick Legrand
//!
//! This module re-exports frequently used types and traits to reduce
//! import boilerplate throughout the codebase.

pub use crate::error::{Error, Result};

// Re-export tracing macros for convenient logging
pub use tracing::{debug, error, info, instrument, trace, warn};

// Re-export serde traits for serialization
pub use serde::{Deserialize, Serialize};

// Re-export common standard library items
pub use std::collections::{HashMap, HashSet};
pub use std::fmt;
pub use std::sync::Arc;

// Re-export async utilities if tokio is enabled
#[cfg(feature = "tokio")]
pub use tokio::sync::{Mutex, RwLock};
