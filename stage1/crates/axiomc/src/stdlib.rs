//! Synthetic stage1 standard library.
//!
//! The AG4.1 milestone introduces a `std.*` surface exposed through the normal
//! `import "std/<module>.ax"` syntax. The compiler materialises a synthetic
//! package under the sentinel path [`STDLIB_ROOT`] whose sources live in a
//! compile-time table instead of the filesystem. Each stdlib module is a thin
//! wrapper around existing capability-gated intrinsics, so capability
//! enforcement continues to run against the importing package's manifest via
//! `hir::lower_with_capabilities`.
//!
//! Today this provides eight stdlib modules. Six are thin wrappers over
//! single-intrinsic capability-gated surfaces, one per capability class:
//!
//! * `std/time.ax` — `now_ms()` on top of `clock_now_ms` (clock).
//! * `std/env.ax` — `get_env(key)` on top of `env_get` (env).
//! * `std/fs.ax` — `read_file(path)` on top of `fs_read` (fs).
//! * `std/net.ax` — `resolve(host)` on top of `net_resolve` (net).
//! * `std/process.ax` — `run_status(command)` on top of `process_status`
//!   (process).
//! * `std/crypto_hash.ax` — `sha256(input)` on top of `crypto_sha256` (crypto).
//!   (This is the stage1 spelling of the `std.crypto.hash` module from the
//!   AG4.1 plan; stage1 uses a flat filename to avoid cross-platform path
//!   separator issues in the virtual stdlib table.)
//!
//! The seventh module shares an existing capability class with a peer
//! wrapper, demonstrating that the `std.*` surface is not limited to one
//! wrapper per capability:
//!
//! * `std/http.ax` — `get(url)` on top of the new `http_get` intrinsic. HTTP
//!   shares the `net` capability surface because any code that can open a
//!   raw TCP socket could implement HTTP itself, so a separate `http`
//!   manifest flag would not add meaningful isolation in stage1. The
//!   stage1 client is http:// only: HTTPS/TLS land in a follow-on slice.
//!
//! The eighth module is the first stdlib surface not tied to a capability
//! flag, matching the ambient status of the `print` statement:
//!
//! * `std/io.ax` — `eprintln(text)` on top of the new ungated `io_eprintln`
//!   intrinsic (writes a line to stderr and returns bytes written).
//!
//! The remaining AG4.1 modules (`std.json`, `std.collections`, `std.sync`)
//! require new stdlib intrinsics, the AG4.2 async runtime, or AG2 generics
//! and land in follow-on slices.

use std::path::{Path, PathBuf};

/// Sentinel path component used as the synthetic stdlib package root.
pub(crate) const STDLIB_ROOT: &str = "<stdlib>";

/// Import-prefix that selects the synthetic stdlib package.
pub(crate) const STDLIB_IMPORT_PREFIX: &str = "std";

/// Package name used for the synthetic stdlib manifest.
pub(crate) const STDLIB_PACKAGE_NAME: &str = "std";

/// Package version used for the synthetic stdlib manifest.
pub(crate) const STDLIB_PACKAGE_VERSION: &str = "0.0.0";

/// Compile-time table of stdlib module sources keyed by their path relative to
/// the stdlib import prefix. Keeping stage1 stdlib sources in-tree as `&str`
/// avoids any filesystem lookup and keeps the bootstrap hermetic.
const STDLIB_SOURCES: &[(&str, &str)] = &[
    (
        "time.ax",
        "pub fn now_ms(): int {\nreturn clock_now_ms()\n}\n",
    ),
    (
        "env.ax",
        "pub fn get_env(key: string): Option<string> {\nreturn env_get(key)\n}\n",
    ),
    (
        "fs.ax",
        "pub fn read_file(path: string): Option<string> {\nreturn fs_read(path)\n}\n",
    ),
    (
        "net.ax",
        "pub fn resolve(host: string): Option<string> {\nreturn net_resolve(host)\n}\n",
    ),
    (
        "process.ax",
        "pub fn run_status(command: string): int {\nreturn process_status(command)\n}\n",
    ),
    (
        "crypto_hash.ax",
        "pub fn sha256(input: string): string {\nreturn crypto_sha256(input)\n}\n",
    ),
    (
        "io.ax",
        "pub fn eprintln(text: string): int {\nreturn io_eprintln(text)\n}\n",
    ),
    (
        "http.ax",
        "pub fn get(url: string): Option<string> {\nreturn http_get(url)\n}\n",
    ),
];

pub(crate) fn stdlib_root() -> PathBuf {
    PathBuf::from(STDLIB_ROOT)
}

pub(crate) fn is_stdlib_path(path: &Path) -> bool {
    path.starts_with(Path::new(STDLIB_ROOT))
}

/// Returns the virtual module path for `module_relative`, e.g.
/// `"time.ax"` -> `<stdlib>/time.ax`.
pub(crate) fn stdlib_source_path(module_relative: &str) -> PathBuf {
    PathBuf::from(STDLIB_ROOT).join(module_relative)
}

/// Returns the embedded source for a virtual stdlib path, or `None` if the
/// path does not correspond to a known stdlib module.
pub(crate) fn stdlib_source_for(path: &Path) -> Option<&'static str> {
    let relative = path.strip_prefix(Path::new(STDLIB_ROOT)).ok()?;
    let key = relative.to_str()?;
    STDLIB_SOURCES
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, source)| *source)
}

/// Returns the virtual module key (e.g. `"time.ax"`) used by a stdlib import
/// remainder. `import_remainder` is the portion of the user-visible import
/// path that follows the `std/` prefix (e.g. `"time.ax"`).
pub(crate) fn stdlib_has_module(import_remainder: &Path) -> bool {
    let Some(key) = import_remainder.to_str() else {
        return false;
    };
    STDLIB_SOURCES.iter().any(|(name, _)| *name == key)
}
