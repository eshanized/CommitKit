# CK - Intelligent Git Commit Assistant

[![Build Status](https://github.com/eshanized/commitkit/workflows/CI/badge.svg)](https://github.com/eshanized/commitkit/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **Author: Eshan Roy**

A production-grade Rust CLI tool for creating high-quality Git commits.

## Features

- **Interactive Commit Builder** - Guided commit creation with live preview
- **Smart Commit Generation** - Automatic commit messages from diff analysis
- **Rule Engine** - Configurable validation with path/branch-based rules
- **Monorepo Support** - Package-aware scoping for large repositories
- **Secret Detection** - Prevent accidental credential leaks
- **Git Hooks** - Native hook management without shell scripts
- **Plugin System** - Extend via WASM plugins

## Installation

```bash
# From source
cargo install --path .

# Or build manually
cargo build --release
```

## Quick Start

```bash
# Interactive commit (default)
ck

# Smart commit from diff
ck smart

# Validate commits
ck check HEAD

# Install git hooks
ck hooks install
```

## Usage

```
Usage: ck [OPTIONS] [COMMAND]

Commands:
  commit        Interactive commit (default)
  smart         Generate commit from diff
  check         Validate commits
  fix           Fix past commits
  hooks         Manage git hooks
  install       Install as git-cz
  version       Print version info

Options:
  -a, --all               Stage modified and deleted files
  --ci                    Enable strict CI mode (no prompts)
  --dry-run               Show result without committing
  --non-interactive       Disable all prompts
  -d, --debug             Enable debug logging
  -h, --help              Print help
  -V, --version           Print version
```

## Configuration

Create a `ck.toml` file in your repository root:

```toml
[rules]
max_subject_length = 72
require_scope = true
allowed_types = ["feat", "fix", "docs", "refactor", "test", "chore"]

[security]
enabled = true
block_on_secret = true
```

See `ck.toml.example` for all available options.

## CI Integration

```yaml
# GitHub Actions
- name: Validate commits
  run: ck check HEAD~10..HEAD --ci
```

## License

MIT License - see [LICENSE](LICENSE) for details.
