//! The pane's own state, and the arithmetic behind it.
//!
//! Nothing here draws or calls CEF, so all of it is unit-testable without a
//! browser in the process: the zoom ladder, the host a zoom is remembered
//! against, the find label, and what a tab's strip title should read.

use std::collections::HashMap;

/// Chrome's zoom ladder, in percent. CEF's own zoom is a continuous level
/// where the scale is `1.2^level`, and haktui's `input::commands::zoom`
/// steps it by half a level, which lands on 109.5% and 131.5%. Chrome shows
/// 110% and 125%, so the pane steps along Chrome's ladder and converts to a
/// CEF level at the edge. Diverged from haktui deliberately; see the PR.
pub const ZOOM_LADDER: [u32; 16] =
    [25, 33, 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400];

pub const ZOOM_DEFAULT: u32 = 100;

/// The ladder step above `percent`, or the top of the ladder.
pub fn zoom_in(percent: u32) -> u32 {
    ZOOM_LADDER.iter().copied().find(|p| *p > percent).unwrap_or(ZOOM_LADDER[ZOOM_LADDER.len() - 1])
}

/// The ladder step below `percent`, or the bottom of the ladder.
pub fn zoom_out(percent: u32) -> u32 {
    ZOOM_LADDER.iter().copied().rev().find(|p| *p < percent).unwrap_or(ZOOM_LADDER[0])
}

/// CEF's zoom level for a percentage: the scale is `1.2^level`, so the level
/// is `log(scale) / log(1.2)`. 100% is level 0 exactly.
pub fn zoom_level(percent: u32) -> f64 {
    if percent == ZOOM_DEFAULT {
        return 0.0;
    }
    (f64::from(percent) / 100.0).ln() / 1.2_f64.ln()
}

/// What the bar shows next to the address, or nothing at 100%.
pub fn zoom_label(percent: u32) -> Option<String> {
    (percent != ZOOM_DEFAULT).then(|| format!("{percent}%"))
}

/// The key a zoom is remembered against: the host, as Chrome does it, so
/// every page on a site opens at the size the last one was read at. A URL
/// with no host (an empty tab) has no key and is not remembered.
pub fn zoom_host(url: &str) -> Option<String> {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// Per-host zoom, the pane's copy of what is written to `ui-settings.json`.
#[derive(Debug, Clone, Default)]
pub struct ZoomMemory {
    by_host: HashMap<String, u32>,
}

impl ZoomMemory {
    pub fn from_settings(saved: &HashMap<String, u32>) -> Self {
        Self { by_host: saved.clone() }
    }

    pub fn as_settings(&self) -> HashMap<String, u32> {
        self.by_host.clone()
    }

    /// The zoom for a URL's host, or 100%.
    pub fn get(&self, url: &str) -> u32 {
        zoom_host(url).and_then(|h| self.by_host.get(&h).copied()).unwrap_or(ZOOM_DEFAULT)
    }

    /// Remember a zoom for a URL's host. 100% is the default, so it is
    /// forgotten rather than written, and the file stays small.
    /// Returns true when the memory changed and settings need a write.
    pub fn set(&mut self, url: &str, percent: u32) -> bool {
        let Some(host) = zoom_host(url) else { return false };
        if percent == ZOOM_DEFAULT {
            return self.by_host.remove(&host).is_some();
        }
        self.by_host.insert(host, percent) != Some(percent)
    }
}

/// What the find field reports: "3 of 12", or nothing before the first
/// result arrives.
pub fn find_label(current: i32, total: i32) -> Option<String> {
    if total <= 0 {
        return Some("No results".to_string());
    }
    Some(format!("{} of {}", current.max(1), total))
}

/// The words on a tab. A page that has not said its title yet shows its
/// host, and a tab with neither reads "New tab" rather than an empty strip.
pub fn tab_title(title: &str, url: &str) -> String {
    let title = title.trim();
    if !title.is_empty() {
        return title.to_string();
    }
    match zoom_host(url) {
        Some(host) => host,
        None => "New tab".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_steps_along_chromes_ladder() {
        assert_eq!(zoom_in(100), 110);
        assert_eq!(zoom_in(110), 125);
        assert_eq!(zoom_out(100), 90);
        assert_eq!(zoom_out(25), 25, "the bottom of the ladder holds");
        assert_eq!(zoom_in(400), 400, "the top of the ladder holds");
        // A percentage that is not on the ladder still steps to a neighbour.
        assert_eq!(zoom_in(105), 110);
        assert_eq!(zoom_out(105), 100);
    }

    #[test]
    fn zoom_level_is_cefs_scale() {
        assert_eq!(zoom_level(100), 0.0);
        // 1.2^level = 1.25 -> level = log(1.25)/log(1.2)
        let level = zoom_level(125);
        assert!((1.2_f64.powf(level) - 1.25).abs() < 1e-9, "level {level} is not 125%");
        assert!(zoom_level(50) < 0.0);
    }

    #[test]
    fn only_a_zoom_that_is_not_100_is_shown() {
        assert_eq!(zoom_label(100), None);
        assert_eq!(zoom_label(125).as_deref(), Some("125%"));
    }

    #[test]
    fn zoom_is_remembered_per_host() {
        let mut memory = ZoomMemory::default();
        assert!(memory.set("https://example.com/a", 125));
        assert_eq!(memory.get("https://example.com/b/c?d=1"), 125);
        assert_eq!(memory.get("https://other.example/"), 100);
        // Back to 100 forgets the host rather than writing a default.
        assert!(memory.set("https://example.com/a", 100));
        assert!(memory.as_settings().is_empty());
    }

    #[test]
    fn a_zoom_host_ignores_scheme_port_and_path() {
        assert_eq!(zoom_host("https://Example.COM:8443/x").as_deref(), Some("example.com"));
        assert_eq!(zoom_host("http://localhost:8457/").as_deref(), Some("localhost"));
        assert_eq!(zoom_host("example.com").as_deref(), Some("example.com"));
        assert_eq!(zoom_host(""), None);
        assert_eq!(zoom_host("https://"), None);
    }

    #[test]
    fn find_says_which_match_of_how_many() {
        assert_eq!(find_label(3, 12).as_deref(), Some("3 of 12"));
        assert_eq!(find_label(0, 0).as_deref(), Some("No results"));
        // CEF reports the ordinal as 0 for the first update of a match run.
        assert_eq!(find_label(0, 5).as_deref(), Some("1 of 5"));
    }

    #[test]
    fn a_tab_falls_back_to_its_host_then_to_new_tab() {
        assert_eq!(tab_title("Example Domain", "https://example.com"), "Example Domain");
        assert_eq!(tab_title("  ", "https://example.com/x"), "example.com");
        assert_eq!(tab_title("", ""), "New tab");
    }
}
