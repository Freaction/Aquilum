# Application checks

`.github/workflows/app-checks.yml` runs on pull requests that change the native Rust/Masonry workspace or its Cargo configuration. On Ubuntu 24.04 it runs `cargo test --workspace --locked` with the Linux libraries required to compile the application.

The gate compiles the active application and runs its workspace tests. It does not build installers: packaging is platform-specific and remains in the release workflow, so pull requests get code feedback without repeating the release matrix.

Path portability regressions are covered in the same suite: Unix rejects Windows drive, rooted, and UNC paths outside a vault; Wikixiv extracts note titles from Windows-style separators; trash restoration compares canonical paths to handle macOS `/var` and `/private/var` aliases.
