//! The build stamp an engine reports in `EngineInfo` and a client compares
//! against its own. Two binaries from the same checkout carry the same sha;
//! commit time orders the rest. Filled in by this crate's build script.

use serde::{Deserialize, Serialize};

/// Short git sha of the checkout, or `unknown` for a build outside git that
/// did not pass `SURYA_BUILD_SHA`.
pub const SHA: &str = env!("SURYA_BUILD_SHA");
/// Commit time as unix seconds, `0` when unknown.
pub const COMMIT_TIME: &str = env!("SURYA_BUILD_COMMIT_TIME");
/// What to do about an older engine, in every message that names one.
pub const UPDATE_HINT: &str = "update it with deploy/install-engine.sh";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub sha: String,
    pub commit_time: i64,
    pub version: String,
}

impl BuildInfo {
    /// `abc123def (0.2.34, 2026-09-05 02:01 UTC)`, or without the date when
    /// the commit time is unknown.
    pub fn label(&self) -> String {
        match chrono::DateTime::from_timestamp(self.commit_time, 0).filter(|_| self.commit_time > 0)
        {
            Some(t) => format!(
                "{} ({}, {} UTC)",
                self.sha,
                self.version,
                t.format("%Y-%m-%d %H:%M")
            ),
            None => format!("{} ({})", self.sha, self.version),
        }
    }
}

/// The stamp of the binary this code is compiled into.
pub fn current() -> BuildInfo {
    BuildInfo {
        sha: SHA.to_string(),
        commit_time: COMMIT_TIME.parse().unwrap_or(0),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// The banner to show when `engine` was built from a different commit than
/// `app`; `None` when they match. An engine without a stamp predates this
/// check and is reported as older.
pub fn skew(app: &BuildInfo, engine: Option<&BuildInfo>) -> Option<String> {
    let Some(engine) = engine else {
        return Some(format!(
            "Engine reports no build stamp, so it is older than this app {}: {UPDATE_HINT}",
            app.label()
        ));
    };
    if engine.sha == app.sha {
        return None;
    }
    let (e, a) = (engine.label(), app.label());
    if engine.commit_time == 0 || app.commit_time == 0 {
        return Some(format!(
            "Engine {e} does not match this app {a}: {UPDATE_HINT}"
        ));
    }
    Some(match engine.commit_time.cmp(&app.commit_time) {
        std::cmp::Ordering::Less => format!("Engine {e} is older than this app {a}: {UPDATE_HINT}"),
        std::cmp::Ordering::Greater => {
            format!("Engine {e} is newer than this app {a}: update the app")
        }
        std::cmp::Ordering::Equal => format!("Engine {e} differs from this app {a}: {UPDATE_HINT}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(sha: &str, t: i64) -> BuildInfo {
        BuildInfo {
            sha: sha.into(),
            commit_time: t,
            version: "0.2.34".into(),
        }
    }

    #[test]
    fn same_sha_is_no_skew() {
        assert_eq!(skew(&stamp("aaa", 10), Some(&stamp("aaa", 10))), None);
    }

    #[test]
    fn missing_stamp_reads_as_older() {
        let m = skew(&stamp("aaa", 10), None).unwrap();
        assert!(
            m.contains("no build stamp") && m.contains("older") && m.contains(UPDATE_HINT),
            "{m}"
        );
    }

    #[test]
    fn older_engine_says_older_with_both_stamps() {
        let m = skew(&stamp("bbb", 20), Some(&stamp("aaa", 10))).unwrap();
        assert!(
            m.starts_with("Engine aaa") && m.contains("older than this app bbb"),
            "{m}"
        );
        assert!(m.ends_with(UPDATE_HINT), "{m}");
    }

    #[test]
    fn newer_engine_says_update_the_app() {
        let m = skew(&stamp("aaa", 10), Some(&stamp("bbb", 20))).unwrap();
        assert!(m.contains("newer") && m.ends_with("update the app"), "{m}");
    }

    #[test]
    fn unknown_time_still_flags_the_mismatch() {
        let m = skew(&stamp("aaa", 10), Some(&stamp("unknown", 0))).unwrap();
        assert!(m.contains("does not match"), "{m}");
    }

    #[test]
    fn current_stamp_is_filled_by_the_build_script() {
        let c = current();
        assert!(!c.sha.is_empty() && !c.version.is_empty());
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"commitTime\""), "{json}");
    }
}
