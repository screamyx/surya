//! Folder paths belong to the browsed device, which may use another OS.

fn windows_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        || path.starts_with("\\\\")
}

fn windows_separators(path: &str) -> String {
    let parts: Vec<_> = path
        .split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .collect();
    let prefix = if path.starts_with("\\\\") || path.starts_with("//") {
        "\\\\"
    } else if path.starts_with(['\\', '/']) {
        "\\"
    } else {
        ""
    };
    let mut normalized = format!("{prefix}{}", parts.join("\\"));
    // A drive root must stay absolute, including when the picker confirms it.
    if parts.last().is_some_and(|part| {
        let bytes = part.as_bytes();
        bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
    }) && path.contains(['\\', '/'])
    {
        normalized.push('\\');
    }
    normalized
}

/// Normalize at the createSpace boundary using the selected device's OS.
/// A Unix filename may contain a literal backslash, so leave Unix paths alone.
pub fn normalize_project_path(path: &str, platform: &str) -> String {
    if platform.eq_ignore_ascii_case("windows") {
        windows_separators(path)
    } else {
        path.to_string()
    }
}

/// Persist a folder-derived project name, interpreting the device's separators.
pub fn project_name(path: &str, platform: &str) -> String {
    let windows = platform.eq_ignore_ascii_case("windows");
    let trimmed = path.trim_end_matches(|c| c == '/' || (windows && c == '\\'));
    if trimmed.is_empty() || (windows && trimmed.ends_with(':')) {
        return path.to_string();
    }
    trimmed
        .rsplit(|c| c == '/' || (windows && c == '\\'))
        .next()
        .unwrap_or(path)
        .to_string()
}

/// Join an engine listing and a folder name without using the client's OS.
pub fn child_path(base: &str, name: &str) -> String {
    if windows_path(base) {
        let base = windows_separators(base);
        format!("{}\\{name}", base.trim_end_matches('\\'))
    } else if base.ends_with('/') {
        format!("{base}{name}")
    } else {
        format!("{base}/{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_listing_joins_use_one_native_separator_on_any_client() {
        for base in ["E:\\", "E:/", "E:\\/"] {
            assert_eq!(child_path(base, "workshop-plans"), "E:\\workshop-plans");
        }
        assert_eq!(child_path("E:\\workshop", "notes"), "E:\\workshop\\notes");
        assert_eq!(
            child_path("\\\\server\\share\\", "notes"),
            "\\\\server\\share\\notes"
        );
    }

    #[test]
    fn submission_normalizes_windows_paths_and_preserves_drive_and_unc_roots() {
        for path in [
            "E:\\/workshop-plans",
            "E:/workshop-plans",
            "E:\\workshop-plans",
        ] {
            assert_eq!(
                normalize_project_path(path, "windows"),
                "E:\\workshop-plans"
            );
        }
        assert_eq!(normalize_project_path("E:/", "windows"), "E:\\");
        assert_eq!(
            normalize_project_path("//server/share/folder", "windows"),
            "\\\\server\\share\\folder"
        );
        assert_eq!(
            normalize_project_path("\\\\server\\share", "windows"),
            "\\\\server\\share"
        );
    }

    #[test]
    fn unix_paths_and_literal_backslashes_are_not_reinterpreted() {
        assert_eq!(child_path("/", "workshop"), "/workshop");
        assert_eq!(
            child_path("/home/user", "back\\slash"),
            "/home/user/back\\slash"
        );
        assert_eq!(
            normalize_project_path("/home/back\\slash", "linux"),
            "/home/back\\slash"
        );
        assert_eq!(
            normalize_project_path("/Users/me/project", "macos"),
            "/Users/me/project"
        );
    }
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn project_names_follow_the_device_path_and_preserve_roots() {
        for (path, platform, expected) in [
            ("E:\\workshop-plans", "windows", "workshop-plans"),
            ("E:\\workshop-plans\\", "windows", "workshop-plans"),
            ("\\\\server\\share\\Notes", "windows", "Notes"),
            ("E:\\", "windows", "E:\\"),
            ("/", "linux", "/"),
            ("/home/back\\slash", "linux", "back\\slash"),
            ("/Users/me/Project Notes/", "macos", "Project Notes"),
        ] {
            assert_eq!(project_name(path, platform), expected);
        }
    }
}
