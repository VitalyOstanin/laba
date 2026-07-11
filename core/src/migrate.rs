//! Forward-only schema migrations for the JSON config/settings files.
//!
//! Each persisted file (`config.json`, `gui-settings.json`) carries a
//! `schema_version`. On load the stored JSON is migrated forward step by step to
//! the current version *before* it is deserialized into the typed struct, so a
//! breaking shape change ships as a `vN -> vN+1` step instead of breaking older
//! files. Absent versions (pre-versioning files) are treated as [`BASE_VERSION`].
//!
//! The load flow (see [`crate::config::Config::load`]) is: parse to
//! [`serde_json::Value`], read the version, run the forward steps, deserialize
//! (which verifies the migrated shape loads), and — only when a step actually
//! ran — back up the original file and rewrite the migrated one. A file newer
//! than this binary is left untouched (never downgraded).

use std::path::Path;

use serde_json::Value;

use crate::error::Error;

/// One migration step: transform the parsed JSON in place, from version N to N+1.
pub type Step = fn(&mut Value) -> Result<(), Error>;

/// The version assumed for a file with no `schema_version` (pre-versioning).
pub const BASE_VERSION: u32 = 1;

/// Read `schema_version` from parsed JSON, clamped to at least [`BASE_VERSION`].
/// An absent, non-numeric, or below-base value reads as [`BASE_VERSION`].
pub fn version_of(value: &Value) -> u32 {
    value
        .get("schema_version")
        .and_then(Value::as_u64)
        .map(|v| v as u32)
        .unwrap_or(BASE_VERSION)
        .max(BASE_VERSION)
}

/// Migrate `value` forward from `from` to `current`. `steps[i]` migrates version
/// `BASE_VERSION + i` to `BASE_VERSION + i + 1`, so `steps.len()` must equal
/// `current - BASE_VERSION`. Returns whether any step ran (i.e. the file should
/// be backed up and rewritten). A version at or above `current` (equal, or from
/// a newer binary) runs nothing and returns `false`; the caller must not rewrite
/// a file that is newer than it understands.
pub fn run(value: &mut Value, from: u32, current: u32, steps: &[Step]) -> Result<bool, Error> {
    debug_assert_eq!(
        steps.len(),
        (current - BASE_VERSION) as usize,
        "migration step count must match the current schema version"
    );
    if from >= current {
        return Ok(false);
    }
    for v in from..current {
        let idx = (v - BASE_VERSION) as usize;
        steps[idx](value)?;
    }
    Ok(true)
}

/// Back up the original file bytes next to it as `<name>.bak-v<from>` before a
/// migrated version overwrites it. A failing backup aborts the migration write
/// so the original is never lost.
pub fn backup(path: &Path, original: &str, from: u32) -> Result<(), Error> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config".to_owned());
    let bak = path.with_file_name(format!("{name}.bak-v{from}"));
    std::fs::write(&bak, original)
        .map_err(|e| Error::Io(format!("write backup {}: {e}", bak.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Synthetic steps exercising the runner independently of any real schema.
    fn step_a(v: &mut Value) -> Result<(), Error> {
        v["a"] = json!(1);
        Ok(())
    }
    fn step_b(v: &mut Value) -> Result<(), Error> {
        v["b"] = json!(2);
        Ok(())
    }

    #[test]
    fn version_of_defaults_and_clamps() {
        assert_eq!(version_of(&json!({})), BASE_VERSION);
        assert_eq!(version_of(&json!({ "schema_version": 0 })), BASE_VERSION);
        assert_eq!(version_of(&json!({ "schema_version": 3 })), 3);
        assert_eq!(version_of(&json!({ "schema_version": "x" })), BASE_VERSION);
    }

    #[test]
    fn runs_only_the_needed_steps() {
        let steps: &[Step] = &[step_a, step_b];
        // from v1 to v3: both steps run.
        let mut v = json!({ "schema_version": 1 });
        assert!(run(&mut v, 1, 3, steps).unwrap());
        assert_eq!(v["a"], json!(1));
        assert_eq!(v["b"], json!(2));

        // from v2 to v3: only the second step runs.
        let mut v = json!({ "schema_version": 2 });
        assert!(run(&mut v, 2, 3, steps).unwrap());
        assert!(v.get("a").is_none());
        assert_eq!(v["b"], json!(2));
    }

    #[test]
    fn at_or_above_current_runs_nothing() {
        let steps: &[Step] = &[step_a, step_b];
        // equal: no migration.
        let mut v = json!({ "schema_version": 3 });
        assert!(!run(&mut v, 3, 3, steps).unwrap());
        assert!(v.get("a").is_none());
        // newer than binary: no migration, no panic.
        let mut v = json!({ "schema_version": 4 });
        assert!(!run(&mut v, 4, 3, steps).unwrap());
    }
}
