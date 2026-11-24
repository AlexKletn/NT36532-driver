# Project instructions

Follow the [code quality agreement](CODE_QUALITY.md) when writing, editing, or reviewing code in this repository.

Follow the [commit message convention](COMMIT_CONVENTIONS.md) when creating commits in this repository.

Keep board-specific wiring and configuration in the board project.

Implement driver logic in Rust. Use thin C shims where the Linux kernel requires C macros, callbacks, or APIs without usable Rust bindings.
