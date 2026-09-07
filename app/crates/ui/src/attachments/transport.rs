use super::format_by_extension;
use std::path::Path;

/// The body used for file-only sends.
pub const FILE_ONLY_TEXT: &str = "See the attached file(s).";

/// Legacy image-only text remains readable in saved conversations.
pub const ATTACHMENT_ONLY_TEXT: &str = "See the attached image(s).";

/// How attachments ride the prompt (use-attachments.ts `withAttachments`):
/// plain local paths appended to the text — the files are staged on the device
/// that runs the agent, so the agent can open them with its own tools; the
/// same text is what persists as the user doc entry.
pub fn with_attachments(text: &str, paths: &[String]) -> String {
    if paths.is_empty() {
        return text.to_string();
    }
    let refs: Vec<String> = paths.iter().map(|p| format!("- {p}")).collect();
    let images_only = paths
        .iter()
        .all(|p| format_by_extension(Path::new(p)).is_some());
    let body = if text.is_empty() {
        if images_only {
            ATTACHMENT_ONLY_TEXT
        } else {
            FILE_ONLY_TEXT
        }
    } else {
        text
    };
    let marker = if images_only {
        "Attached images (local files — open them to view):"
    } else {
        "Attached files (local files, open them to read or view):"
    };
    format!("{body}\n\n{marker}\n{}", refs.join("\n"))
}

/// An attachment ref parsed back out of a user message's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserImageAttachment {
    pub id: String,
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUserMessage {
    /// The visible prompt (the refs trailer stripped; empty for image-only sends).
    pub text: String,
    pub attachments: Vec<UserImageAttachment>,
}

pub(super) fn name_from_path(path: &str) -> String {
    let name = path
        .rsplit(['/', '\\'])
        .next()
        .map(str::trim)
        .unwrap_or_default();
    if name.is_empty() {
        "image".to_string()
    } else {
        name.to_string()
    }
}

/// Find the refs trailer: a blank line, then a line starting (case-insensitive)
/// with `Attached images (local files` and ending `):`. Returns
/// `(body_end, refs_start)` byte offsets — the tolerant equivalent of surya's
/// `ATTACHED_IMAGES_RE`.
fn find_refs_marker(content: &str) -> Option<(usize, usize)> {
    let lower = content.to_ascii_lowercase();
    let needles = [
        "\n\nattached images (local files",
        "\n\nattached files (local files",
    ];
    let mut from = 0usize;
    while let Some(rel) = needles
        .iter()
        .filter_map(|needle| lower[from..].find(needle))
        .min()
    {
        let gap = from + rel;
        let line_start = gap + 2;
        let line_end = content[line_start..]
            .find('\n')
            .map(|p| line_start + p)
            .unwrap_or(content.len());
        let line = content[line_start..line_end].trim_end_matches('\r');
        if line.ends_with("):") {
            let refs_start = (line_end + 1).min(content.len());
            return Some((gap, refs_start));
        }
        from = line_start;
    }
    None
}

/// message-attachments.ts `parseUserMessageImages`: split the visible prompt
/// from its attachment-ref trailer.
pub fn parse_user_message_images(content: &str) -> ParsedUserMessage {
    let Some((body_end, refs_start)) = find_refs_marker(content) else {
        return ParsedUserMessage {
            text: content.to_string(),
            attachments: Vec::new(),
        };
    };
    let body = content[..body_end].trim_end();
    let attachments: Vec<UserImageAttachment> = content[refs_start..]
        .lines()
        .filter_map(|line| {
            let path = line.trim_start().strip_prefix("- ")?.trim();
            (!path.is_empty()).then(|| path.to_string())
        })
        .enumerate()
        .map(|(index, path)| UserImageAttachment {
            id: format!("{index}:{path}"),
            name: name_from_path(&path),
            path,
        })
        .collect();
    if attachments.is_empty() {
        return ParsedUserMessage {
            text: content.to_string(),
            attachments,
        };
    }
    ParsedUserMessage {
        text: if body.trim() == ATTACHMENT_ONLY_TEXT || body.trim() == FILE_ONLY_TEXT {
            String::new()
        } else {
            body.to_string()
        },
        attachments,
    }
}

/// message-attachments.ts `userMessageRailText`: what the rail/sidebar shows
/// for a user message ("Attached image" / "N attached images" when image-only).
pub fn user_message_rail_text(content: &str) -> String {
    let parsed = parse_user_message_images(content);
    if !parsed.text.trim().is_empty() {
        return parsed.text;
    }
    let images_only = parsed
        .attachments
        .iter()
        .all(|a| format_by_extension(Path::new(&a.path)).is_some());
    if !images_only {
        return match parsed.attachments.len() {
            1 => "Attached file".into(),
            n => format!("{n} attached files"),
        };
    }
    match parsed.attachments.len() {
        0 => content.to_string(),
        1 => "Attached image".to_string(),
        n => format!("{n} attached images"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_and_mixed_attachments_round_trip_without_image_wording() {
        for paths in [
            vec!["/uploads/notes.txt".into()],
            vec!["pending://a/notes.txt".into(), "pending://b/pic.png".into()],
        ] {
            let content = with_attachments("Read the note", &paths);
            assert!(content.contains("Attached files"));
            let parsed = parse_user_message_images(&content);
            assert_eq!(parsed.text, "Read the note");
            assert_eq!(
                parsed
                    .attachments
                    .iter()
                    .map(|a| a.path.clone())
                    .collect::<Vec<_>>(),
                paths
            );
        }
    }
    #[test]
    fn file_only_prompt_and_sidebar_use_file_wording() {
        let content = with_attachments("", &["/uploads/notes.txt".into()]);
        assert!(content.starts_with(FILE_ONLY_TEXT));
        assert!(parse_user_message_images(&content).text.is_empty());
        assert_eq!(user_message_rail_text(&content), "Attached file");
    }
}
