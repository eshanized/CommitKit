// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Commit module for message handling and interactive building.

mod builder;
pub mod fix;
mod message;
mod preview;

pub use builder::CommitBuilder;
pub use message::CommitMessage;
pub use preview::CommitPreview;
