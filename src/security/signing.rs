// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Commit signing verification.

use crate::error::Result;

/// Status of commit signing.
#[derive(Debug, Clone)]
pub enum SigningStatus {
    /// Commit is signed with a valid signature.
    Signed { signer: Option<String> },
    /// Commit is not signed.
    Unsigned,
    /// Signature verification failed.
    Invalid { reason: String },
    /// Unable to verify (missing keys, etc.).
    Unknown { reason: String },
}

impl SigningStatus {
    /// Check if the commit is properly signed.
    pub fn is_signed(&self) -> bool {
        matches!(self, SigningStatus::Signed { .. })
    }

    /// Get a human-readable description.
    pub fn description(&self) -> String {
        match self {
            SigningStatus::Signed { signer: Some(s) } => {
                format!("Signed by {}", s)
            }
            SigningStatus::Signed { signer: None } => "Signed".to_string(),
            SigningStatus::Unsigned => "Unsigned".to_string(),
            SigningStatus::Invalid { reason } => {
                format!("Invalid signature: {}", reason)
            }
            SigningStatus::Unknown { reason } => {
                format!("Cannot verify: {}", reason)
            }
        }
    }
}

/// Check the signing status of a commit.
pub fn check_signing_status(reference: &str) -> Result<SigningStatus> {
    // Use git command to check signature
    let output = std::process::Command::new("git")
        .args(["verify-commit", "--raw", reference])
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                // Extract signer from output if possible
                let stderr = String::from_utf8_lossy(&output.stderr);
                let signer = extract_signer_from_gpg_output(&stderr);

                Ok(SigningStatus::Signed { signer })
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);

                if stderr.contains("no signature found") {
                    Ok(SigningStatus::Unsigned)
                } else if stderr.contains("BAD signature") {
                    Ok(SigningStatus::Invalid {
                        reason: "Bad signature".to_string(),
                    })
                } else if stderr.contains("key") {
                    Ok(SigningStatus::Unknown {
                        reason: "Missing public key".to_string(),
                    })
                } else {
                    Ok(SigningStatus::Unknown {
                        reason: stderr.to_string(),
                    })
                }
            }
        }
        Err(e) => Ok(SigningStatus::Unknown {
            reason: format!("Git command failed: {}", e),
        }),
    }
}

/// Extract signer name from GPG output.
fn extract_signer_from_gpg_output(output: &str) -> Option<String> {
    // Look for "GOODSIG" line which contains the signer
    for line in output.lines() {
        if line.contains("GOODSIG") {
            // Format: [GNUPG:] GOODSIG <keyid> <name>
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                return Some(parts[3..].join(" "));
            }
        }
    }
    None
}

/// Check if the current git config has signing enabled.
#[allow(dead_code)]
pub fn is_signing_configured() -> bool {
    let output = std::process::Command::new("git")
        .args(["config", "--get", "commit.gpgsign"])
        .output();

    match output {
        Ok(output) => {
            let value = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_lowercase();
            value == "true"
        }
        Err(_) => false,
    }
}

/// Get the signing key configured in git.
#[allow(dead_code)]
pub fn get_signing_key() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["config", "--get", "user.signingkey"])
        .output()
        .ok()?;

    if output.status.success() {
        let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !key.is_empty() {
            return Some(key);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signing_status_description() {
        let signed = SigningStatus::Signed {
            signer: Some("John Doe".to_string()),
        };
        assert!(signed.description().contains("John Doe"));
        assert!(signed.is_signed());

        let unsigned = SigningStatus::Unsigned;
        assert_eq!(unsigned.description(), "Unsigned");
        assert!(!unsigned.is_signed());
    }

    #[test]
    fn test_extract_signer() {
        let output = "[GNUPG:] GOODSIG ABCD1234 John Doe <john@example.com>";
        let signer = extract_signer_from_gpg_output(output);
        assert!(signer.is_some());
        assert!(signer.unwrap().contains("John"));
    }
}
