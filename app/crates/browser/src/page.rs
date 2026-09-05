//! The half of a browser that is the window rather than the page: what the
//! address bar shows. Ported from haktui's `chrome.rs`, trimmed to one page.

use std::sync::Mutex;

/// Everything the address bar reads. One page, one browser.
#[derive(Clone, Debug, Default)]
pub struct Page {
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub can_back: bool,
    pub can_forward: bool,
    /// The address Enter asked for, until the page commits to one. Chrome
    /// keeps the typed address in the bar while the site is still answering.
    pub pending: Option<String>,
    /// Why the last navigation failed, in plain words. Cleared by the next
    /// navigation or the next committed address.
    pub error: Option<String>,
    /// How far the load is, 0 to 1.
    pub progress: f32,
}

impl Page {
    pub fn shown_address(&self) -> &str {
        self.pending.as_deref().unwrap_or(&self.url)
    }

    pub fn begin_navigation(&mut self, url: &str) {
        self.pending = Some(url.to_string());
        self.error = None;
        self.loading = true;
        self.progress = 0.0;
    }

    pub fn committed(&mut self, url: String) {
        self.url = url;
        self.pending = None;
        self.error = None;
    }

    pub fn progressed(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0) as f32;
    }

    pub fn failed(&mut self, failed_url: &str, text: &str) {
        let what = if text.is_empty() { "did not load" } else { text };
        self.error = Some(format!("{failed_url}: {what}"));
        self.pending = None;
        self.loading = false;
        self.progress = 1.0;
    }
}

static PAGE: Mutex<Page> = Mutex::new(Page {
    url: String::new(),
    title: String::new(),
    loading: false,
    can_back: false,
    can_forward: false,
    pending: None,
    error: None,
    progress: 0.0,
});

pub fn page() -> Page {
    PAGE.lock().map(|p| p.clone()).unwrap_or_default()
}

pub(crate) fn update_page(f: impl FnOnce(&mut Page)) {
    if let Ok(mut p) = PAGE.lock() {
        f(&mut p);
    }
}

/// What a typed address means. `http` and `https` are taken as written and
/// any other scheme is refused (empty string): `file://` would read the
/// disk into a pane an agent can drive, `chrome://` is Chrome's own
/// settings. A bare host gets `https://`; anything with a space or no dot
/// is a search.
pub fn navigate_to(typed: &str) -> String {
    let t = typed.trim();
    if t.is_empty() {
        return String::new();
    }
    if let Some((scheme, _)) = t.split_once("://") {
        let scheme = scheme.to_ascii_lowercase();
        return if scheme == "http" || scheme == "https" { t.to_string() } else { String::new() };
    }
    if t.contains(':') && !t.contains('.') && !t.starts_with("localhost") {
        // `about:blank`, `chrome:settings`, `javascript:...`: refused too.
        return String::new();
    }
    if t.starts_with("localhost") || t.starts_with("127.") {
        return format!("http://{t}");
    }
    let looks_like_host = !t.contains(' ') && t.contains('.');
    if looks_like_host {
        format!("https://{t}")
    } else {
        format!("https://duckduckgo.com/?q={}", urlencode(t))
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_addresses_become_urls() {
        assert_eq!(navigate_to("example.com"), "https://example.com");
        assert_eq!(navigate_to("http://a.b/c"), "http://a.b/c");
        assert_eq!(navigate_to("HTTPS://a.b/c"), "HTTPS://a.b/c");
        assert_eq!(navigate_to("localhost:8457"), "http://localhost:8457");
        assert_eq!(navigate_to("two words"), "https://duckduckgo.com/?q=two+words");
        assert_eq!(navigate_to("  "), "");
    }

    #[test]
    fn only_http_and_https_are_taken_as_written() {
        assert_eq!(navigate_to("file:///etc/passwd"), "");
        assert_eq!(navigate_to("chrome://settings"), "");
        assert_eq!(navigate_to("about:blank"), "");
        assert_eq!(navigate_to("javascript:alert(1)"), "");
        assert_eq!(navigate_to("ftp://x.y/z"), "");
    }

    #[test]
    fn pending_shows_until_committed() {
        let mut p = Page::default();
        p.committed("https://a".into());
        p.begin_navigation("https://b");
        assert_eq!(p.shown_address(), "https://b");
        p.committed("https://b/".into());
        assert_eq!(p.shown_address(), "https://b/");
        assert!(p.pending.is_none());
    }
}
