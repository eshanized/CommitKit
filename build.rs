// Author: Eshan Roy
// SPDX-License-Identifier: MIT

use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    EmitBuilder::builder()
        .git_sha(true)
        .git_commit_date()
        .emit()?;
    Ok(())
}
