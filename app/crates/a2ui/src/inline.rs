//! The "simple Markdown" A2UI's `Text` allows (no HTML, images or links):
//! bold, italic and inline code, plus a leading heading marker that the
//! `variant` already expresses. Unmatched markers stay literal. This is a
//! deliberately tiny scanner — comet's full markdown stack lives in the UI
//! crate above this one and would make a dependency cycle.

use gpui::{FontStyle, FontWeight, Hsla, SharedString, StyledText, TextRun, font};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
}

/// Strip a leading `#`-heading marker; the variant carries the size.
pub fn strip_heading(text: &str) -> &str {
    let trimmed = text.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes > 0 && hashes <= 6 && trimmed[hashes..].starts_with(' ') {
        trimmed[hashes..].trim_start()
    } else {
        text
    }
}

/// Scan `text` into styled runs.
pub fn parse_inline(text: &str) -> Vec<Run> {
    let text = strip_heading(text);
    let mut runs: Vec<Run> = Vec::new();
    let mut cur = Run::default();
    let mut bold = false;
    let mut italic = false;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let flush = |runs: &mut Vec<Run>, cur: &mut Run, bold: bool, italic: bool| {
        if !cur.text.is_empty() {
            runs.push(std::mem::take(cur));
        }
        cur.bold = bold;
        cur.italic = italic;
        cur.code = false;
    };
    while i < chars.len() {
        let c = chars[i];
        // Inline code: a backtick run up to its closing twin.
        if c == '`' {
            if let Some(close) = chars[i + 1..].iter().position(|&x| x == '`') {
                flush(&mut runs, &mut cur, bold, italic);
                runs.push(Run {
                    text: chars[i + 1..i + 1 + close].iter().collect(),
                    bold,
                    italic,
                    code: true,
                });
                i += close + 2;
                continue;
            }
        }
        // Bold: `**` or `__` toggles when a closing twin exists.
        if (c == '*' || c == '_') && chars.get(i + 1) == Some(&c) {
            let marker = [c, c];
            let has_close = bold || find_pair(&chars[i + 2..], &marker);
            if has_close {
                flush(&mut runs, &mut cur, !bold, italic);
                bold = !bold;
                i += 2;
                continue;
            }
        }
        // Italic: a single `*` or `_` with a closing twin, not mid-word for `_`.
        if (c == '*' || c == '_')
            && (c == '*' || i == 0 || !chars[i - 1].is_alphanumeric())
        {
            let has_close = italic || chars[i + 1..].iter().any(|&x| x == c);
            if has_close && (italic || chars.get(i + 1).is_some_and(|n| !n.is_whitespace())) {
                flush(&mut runs, &mut cur, bold, !italic);
                italic = !italic;
                i += 1;
                continue;
            }
        }
        cur.text.push(c);
        i += 1;
    }
    if !cur.text.is_empty() {
        runs.push(cur);
    }
    runs
}

fn find_pair(chars: &[char], marker: &[char; 2]) -> bool {
    chars.windows(2).any(|w| w == marker)
}

/// The fonts and colors runs are painted with.
pub struct InkStyle {
    pub sans: SharedString,
    pub mono: SharedString,
    pub color: Hsla,
    pub code_color: Hsla,
    pub code_wash: Hsla,
    pub weight: FontWeight,
}

/// Build a [`StyledText`] from runs — one `TextRun` per style change.
pub fn styled_text(runs: &[Run], style: &InkStyle) -> StyledText {
    let mut text = String::new();
    let mut text_runs: Vec<TextRun> = Vec::with_capacity(runs.len());
    for run in runs {
        if run.text.is_empty() {
            continue;
        }
        text.push_str(&run.text);
        let mut f = font(if run.code {
            style.mono.clone()
        } else {
            style.sans.clone()
        });
        f.weight = if run.bold && style.weight.0 < FontWeight::SEMIBOLD.0 {
            FontWeight::SEMIBOLD
        } else {
            style.weight
        };
        f.style = if run.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };
        text_runs.push(TextRun {
            len: run.text.len(),
            font: f,
            color: if run.code { style.code_color } else { style.color },
            background_color: run.code.then_some(style.code_wash),
            underline: None,
            strikethrough: None,
        });
    }
    StyledText::new(SharedString::from(text)).with_runs(text_runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> Vec<(String, bool, bool, bool)> {
        parse_inline(text)
            .into_iter()
            .map(|r| (r.text, r.bold, r.italic, r.code))
            .collect()
    }

    #[test]
    fn bold_italic_code() {
        assert_eq!(
            plain("a **b** *c* `d`"),
            vec![
                ("a ".into(), false, false, false),
                ("b".into(), true, false, false),
                (" ".into(), false, false, false),
                ("c".into(), false, true, false),
                (" ".into(), false, false, false),
                ("d".into(), false, false, true),
            ]
        );
    }

    #[test]
    fn unmatched_markers_stay_literal() {
        assert_eq!(plain("2 * 3 = 6"), vec![("2 * 3 = 6".into(), false, false, false)]);
        assert_eq!(plain("snake_case_name"), vec![("snake_case_name".into(), false, false, false)]);
        assert_eq!(plain("`open"), vec![("`open".into(), false, false, false)]);
    }

    #[test]
    fn heading_marker_is_stripped() {
        assert_eq!(strip_heading("## Contact Us"), "Contact Us");
        assert_eq!(strip_heading("#hashtag"), "#hashtag");
    }
}
