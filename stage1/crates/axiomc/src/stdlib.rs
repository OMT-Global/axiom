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
//! Today this only provides `std/time.ax`, exposing `now_ms()` on top of the
//! existing `clock_now_ms` intrinsic. Additional AG4.1 modules land in
//! follow-on slices.

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
const STDLIB_SOURCES: &[(&str, &str)] = &[(
    "time.ax",
    "pub fn now_ms(): int {\nreturn clock_now_ms()\n}\n",
)];

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
