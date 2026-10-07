//! Unicode-safe Markdown toolbar edits. Ranges are character offsets, as used by egui.
#[derive(Clone, Copy)]
pub enum Format {
    Bold,
    Italic,
    Heading,
    Task,
    Bullet,
    Numbered,
    Link,
    Code,
}
pub fn apply(text: &mut String, start: usize, end: usize, format: Format) -> (usize, usize) {
    let len = text.chars().count();
    let start = start.min(len);
    let end = end.max(start).min(len);
    let byte = |n| {
        text.char_indices()
            .nth(n)
            .map(|(i, _)| i)
            .unwrap_or(text.len())
    };
    let (a, b) = (byte(start), byte(end));
    if matches!(format, Format::Bullet | Format::Numbered | Format::Task) {
        let from_byte = text[..a].rfind('\n').map_or(0, |n| n + 1);
        let selection_end = if b > a && text[..b].ends_with('\n') {
            b - 1
        } else {
            b
        };
        let to_byte = text[selection_end..]
            .find('\n')
            .map_or(text.len(), |n| selection_end + n);
        let content = &text[from_byte..to_byte];
        let empty = content.is_empty();
        let lines: Vec<_> = if empty {
            vec!["List item"]
        } else {
            content.split('\n').collect()
        };
        let replacement = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let prefix = match format {
                    Format::Bullet => "- ".to_owned(),
                    Format::Task => "- [ ] ".to_owned(),
                    _ => format!("{}. ", i + 1),
                };
                let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
                let indent = &line[..indent_len];
                let body = &line[indent_len..];
                let body = list_marker(body).map_or(body, |(n, _)| &body[n..]);
                format!("{indent}{prefix}{body}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let from = text[..from_byte].chars().count();
        let to = from + replacement.chars().count();
        let selected_from = if empty { to - "List item".len() } else { from };
        text.replace_range(from_byte..to_byte, &replacement);
        return (selected_from, to);
    }
    let selected = &text[a..b];
    let (prefix, suffix, placeholder) = match format {
        Format::Bold => ("**", "**", "bold text"),
        Format::Italic => ("*", "*", "italic text"),
        Format::Heading => ("## ", "", "Heading"),
        Format::Task | Format::Bullet | Format::Numbered => unreachable!(),
        Format::Link => ("[", "](https://example.com)", "link text"),
        Format::Code => ("`", "`", "code"),
    };
    let content = if selected.is_empty() {
        placeholder
    } else {
        selected
    };
    let line_start = if matches!(format, Format::Heading | Format::Task)
        && a > 0
        && !text[..a].ends_with('\n')
    {
        "\n"
    } else {
        ""
    };
    let replacement = format!("{line_start}{prefix}{content}{suffix}");
    let from = start + line_start.chars().count() + prefix.chars().count();
    let to = from + content.chars().count();
    text.replace_range(a..b, &replacement);
    (from, to)
}
// Returns marker byte length and the marker for the next item.
fn list_marker(line: &str) -> Option<(usize, String)> {
    for marker in ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "+ [ ] "] {
        if line.starts_with(marker) {
            return Some((marker.len(), "- [ ] ".into()));
        }
    }
    for marker in ["- ", "* ", "+ "] {
        if line.starts_with(marker) {
            return Some((2, marker.into()));
        }
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0
        && digits <= 9
        && (line[digits..].starts_with(". ") || line[digits..].starts_with(") "))
    {
        let n: u64 = line[..digits].parse().ok()?;
        return Some((
            digits + 2,
            format!("{}{} ", n + 1, &line[digits..digits + 1]),
        ));
    }
    None
}

/// Continue a list at an unselected cursor, or remove an empty item to end it.
/// Returns None when Enter should be handled by the normal text editor.
pub fn list_enter(text: &mut String, cursor: usize) -> Option<usize> {
    let at = text
        .char_indices()
        .nth(cursor)
        .map_or(text.len(), |(i, _)| i);
    let start = text[..at].rfind('\n').map_or(0, |n| n + 1);
    let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
    // Do not autoformat literal code fences.
    let mut fence: Option<&str> = None;
    for line in text[..start].lines() {
        let line = line.trim_start();
        if line.starts_with("```") || line.starts_with("~~~") {
            let marker = &line[..3];
            if fence == Some(marker) {
                fence = None;
            } else if fence.is_none() {
                fence = Some(marker);
            }
        }
    }
    if fence.is_some() {
        return None;
    }
    let line = &text[start..end];
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (marker_len, next) = list_marker(&line[indent_len..])?;
    if at < start + indent_len + marker_len {
        return None;
    }
    if line[indent_len + marker_len..].trim().is_empty() {
        let pos = text[..start].chars().count();
        text.replace_range(start..end, "");
        return Some(pos);
    }
    let insert = format!("\n{}{next}", &line[..indent_len]);
    let pos = cursor + insert.chars().count();
    text.insert_str(at, &insert);
    Some(pos)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_selection_and_empty_cursor() {
        let mut s = "日本語 café".to_owned();
        let range = apply(&mut s, 4, 8, Format::Bold);
        assert_eq!(s, "日本語 **café**");
        assert_eq!(range, (6, 10));
        let mut s = String::new();
        let range = apply(&mut s, 0, 0, Format::Link);
        assert_eq!(s, "[link text](https://example.com)");
        assert_eq!(range, (1, 10));
    }
}

#[cfg(test)]
mod list_tests {
    use super::*;
    #[test]
    fn selected_lines_are_numbered_without_touching_next_line() {
        let mut s = "日本語\ncafé\nKeep".to_owned();
        let range = apply(&mut s, 1, 9, Format::Numbered);
        assert_eq!(s, "1. 日本語\n2. café\nKeep");
        assert_eq!(range, (0, 14));
    }
    #[test]
    fn cursor_formats_whole_line_and_empty_task_selects_placeholder() {
        let mut s = "one\ntwo".to_owned();
        apply(&mut s, 5, 5, Format::Bullet);
        assert_eq!(s, "one\n- two");
        let mut s = String::new();
        assert_eq!(apply(&mut s, 0, 0, Format::Task), (6, 15));
        assert_eq!(s, "- [ ] List item");
    }
}

#[cfg(test)]
mod list_regressions {
    use super::*;
    #[test]
    fn changing_style_replaces_markers_and_preserves_indent() {
        let mut s = "- one\n- [x] café\n  3. nested".to_owned();
        let n = s.chars().count();
        apply(&mut s, 0, n, Format::Numbered);
        assert_eq!(s, "1. one\n2. café\n  3. nested");
        let n = s.chars().count();
        apply(&mut s, 0, n, Format::Bullet);
        assert_eq!(s, "- one\n- café\n  - nested");
    }
    #[test]
    fn enter_continues_numbers_tasks_and_ends_empty_item() {
        let mut s = "9. 日本語".to_owned();
        assert_eq!(list_enter(&mut s, 6), Some(11));
        assert_eq!(s, "9. 日本語\n10. ");
        assert_eq!(list_enter(&mut s, 11), Some(7));
        assert_eq!(s, "9. 日本語\n");
        let mut s = "  - [x] done".to_owned();
        let n = s.chars().count();
        list_enter(&mut s, n);
        assert_eq!(s, "  - [x] done\n  - [ ] ");
    }
    #[test]
    fn enter_ignores_plain_text_and_fenced_code() {
        for source in ["plain", "```md\n- literal", "~~~\n1. literal"] {
            let mut s = source.to_owned();
            let n = s.chars().count();
            assert_eq!(list_enter(&mut s, n), None);
            assert_eq!(s, source);
        }
    }
}
