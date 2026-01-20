// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Security module for secret detection and signing.

mod secrets;
mod signing;

pub use secrets::{detect_secrets, SecretMatch, SecretScanner};
pub use signing::{check_signing_status, SigningStatus};
