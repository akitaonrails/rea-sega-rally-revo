//! Section-aware editing of the game's `config.ini`, ported from the
//! OutRun 2006 launcher's editor: every comment, unmanaged key, section and
//! the file's newline shape (CRLF or LF, trailing newline or not) stay as
//! the player or the game left them. Sections and keys match without
//! regard to case or the spaces around `=`.

fn is_header(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('[') && t.ends_with(']')
}

fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with(';') || t.starts_with('#')
}

/// The key of an active `key = value` line, trimmed.
fn key_of(line: &str) -> Option<&str> {
    if is_header(line) || is_comment(line) {
        return None;
    }
    line.split_once('=').map(|(key, _)| key.trim())
}

/// Sets `key` inside `[section]` as `key=value` (the game's own spelling):
/// the first matching line is replaced in place, later duplicates in the
/// section are dropped (an older launcher appended some), a missing key is
/// inserted at the section's end before its trailing blank lines, and a
/// missing section is appended to the file.
pub fn set_section_key(text: &str, section: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let entry = format!("{key}={value}");
    let header = format!("[{section}]");
    let start = lines
        .iter()
        .position(|line| is_header(line) && line.trim().eq_ignore_ascii_case(&header));
    let Some(start) = start else {
        if lines.last().is_some_and(|line| !line.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(header);
        lines.push(entry);
        return join(lines, text);
    };
    let end = lines[start + 1..]
        .iter()
        .position(|line| is_header(line))
        .map_or(lines.len(), |n| start + 1 + n);
    let matches: Vec<usize> = (start + 1..end)
        .filter(|&n| key_of(&lines[n]).is_some_and(|k| k.eq_ignore_ascii_case(key)))
        .collect();
    if let Some((&first, rest)) = matches.split_first() {
        lines[first] = entry;
        for &n in rest.iter().rev() {
            lines.remove(n);
        }
    } else {
        let mut at = end;
        while at > start + 1 && lines[at - 1].trim().is_empty() {
            at -= 1;
        }
        lines.insert(at, entry);
    }
    join(lines, text)
}

/// Rebuilds the file with the original's line ending and trailing-newline
/// shape (an empty file gains a newline).
fn join(lines: Vec<String>, original: &str) -> String {
    let eol = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut out = lines.join(eol);
    if original.is_empty() || original.ends_with('\n') {
        out.push_str(eol);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "; player file\n[Audio]\nQuality=2\n\n[Graphics]\n# note\nHorizontalResolution = 1280\nCustomKey=keep\n\n[GameController]\nGUID={1234}\n";

    #[test]
    fn replaces_a_spaced_key_in_place() {
        let out = set_section_key(FILE, "Graphics", "HorizontalResolution", "3840");
        assert!(out.contains("# note\nHorizontalResolution=3840\nCustomKey=keep\n"));
        assert_eq!(out.matches("HorizontalResolution").count(), 1);
        assert_eq!(out.lines().count(), FILE.lines().count());
    }

    #[test]
    fn keys_and_sections_match_without_case() {
        let out = set_section_key(
            "[graphics]\nhorizontalresolution=800\n",
            "Graphics",
            "HorizontalResolution",
            "1920",
        );
        assert_eq!(out, "[graphics]\nHorizontalResolution=1920\n");
    }

    #[test]
    fn other_sections_and_comments_are_untouched() {
        let out = set_section_key(FILE, "Graphics", "Quality", "9");
        assert!(out.contains("[Audio]\nQuality=2\n"));
        assert!(out.contains("; player file\n"));
        assert!(out.contains("[GameController]\nGUID={1234}\n"));
        // inserted at the end of [Graphics], before its blank line
        assert!(out.contains("CustomKey=keep\nQuality=9\n\n[GameController]"));
        // a commented-out key is never matched
        let out = set_section_key("[Graphics]\n;VSync=0\n", "Graphics", "VSync", "1");
        assert_eq!(out, "[Graphics]\n;VSync=0\nVSync=1\n");
    }

    #[test]
    fn duplicates_left_by_older_launchers_are_dropped() {
        let text = "[Graphics]\nWidth = 1280\nOther=1\nWidth=3840\n[Audio]\nWidth=5\n";
        let out = set_section_key(text, "Graphics", "Width", "2560");
        assert_eq!(out, "[Graphics]\nWidth=2560\nOther=1\n[Audio]\nWidth=5\n");
    }

    #[test]
    fn missing_sections_are_appended() {
        assert_eq!(
            set_section_key("[Audio]\nQuality=3\n", "Graphics", "VSync", "1"),
            "[Audio]\nQuality=3\n\n[Graphics]\nVSync=1\n"
        );
        assert_eq!(
            set_section_key("", "Graphics", "VSync", "1"),
            "[Graphics]\nVSync=1\n"
        );
    }

    #[test]
    fn newline_shape_is_kept() {
        let crlf = "[Graphics]\r\nVSync=0\r\n";
        assert_eq!(
            set_section_key(crlf, "Graphics", "VSync", "1"),
            "[Graphics]\r\nVSync=1\r\n"
        );
        let bare = "[Graphics]\nVSync=0";
        assert_eq!(
            set_section_key(bare, "Graphics", "RunWindowed", "0"),
            "[Graphics]\nVSync=0\nRunWindowed=0"
        );
    }
}
