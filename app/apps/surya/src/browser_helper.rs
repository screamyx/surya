//! `surya-browser-helper`: CEF's subprocess. Chromium starts one of these
//! per renderer, GPU process and utility. It hands the process to CEF and
//! exits with whatever CEF says; nothing of surya runs in it.

fn main() {
    surya_browser::helper_main();
}
