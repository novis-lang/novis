//! What a doc comment's `@see` names, and the headings a Markdown file can be
//! pointed at (`rule:tooling/doc-comment-tags-are-closed-and-checked`).
//!
//! **Two shapes, told apart by the text alone.** A target containing `/` or `.`
//! is a file, written relative to the folder of the file that wrote it, with an
//! optional `#anchor` after it. Every other target is a class or a member,
//! `Class::member`. A class or member name never contains either character, so
//! the two shapes never overlap and nothing has to be resolved to tell them
//! apart. [`DocTarget::of`] is the one classifier: the checker, the language
//! server, `nvs pkg pack`, `nvs meta` and `nvs doc` all call it, so they cannot
//! disagree about which shape a tag has.
//!
//! **An anchor names a heading by its slug**, which [`heading_slugs`] computes
//! with GitHub's rules, the ones VS Code's Markdown preview shares, so a link a
//! reader follows in either lands where the check said it would. The scan reads
//! lines and depends on no Markdown crate: the stdlib's parser is a runtime
//! crate, and headings are all the check needs.
//!
//! Nothing here touches the disk. Whether a path stays inside the project and
//! names a file is the checker's question (`nvs_hir::members`).

/// One `@see` argument, classified by [`DocTarget::of`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocTarget<'a> {
    /// A class, or a member of one: `Core\Money::fromCents`, `self::$name`.
    Member(&'a str),
    /// A file, relative to the folder of the file that wrote the tag.
    File {
        /// The written path, without its anchor.
        path: &'a str,
        /// The text after the first `#`, if one was written. It may be empty.
        anchor: Option<&'a str>,
    },
}

impl<'a> DocTarget<'a> {
    /// Classifies a `@see` argument. The text is trimmed first.
    #[must_use]
    pub fn of(text: &'a str) -> Self {
        let text = text.trim();
        if !text.contains(['/', '.']) {
            return Self::Member(text);
        }
        match text.split_once('#') {
            Some((path, anchor)) => Self::File {
                path,
                anchor: Some(anchor),
            },
            None => Self::File {
                path: text,
                anchor: None,
            },
        }
    }
}

/// Whether `path` names a Markdown file, the only kind an anchor may point
/// into: its extension is `.md` or `.markdown`, in any case.
#[must_use]
pub fn is_markdown(path: &str) -> bool {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty() && (ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
    })
}

/// Every heading of the Markdown `text` as its anchor slug and its 0-based
/// line, in document order.
///
/// A heading is an ATX line (`#` to `######`, then a space or the line's end)
/// or a setext one (a paragraph line underlined with `=` or `-`). A line
/// inside a fenced code block (` ``` ` or `~~~`) is never one. The slug is
/// GitHub's: the text lowercased, every character that is not a letter, a
/// digit, a space, `-` or `_` removed, and each space turned into `-`. A slug
/// seen before gets `-1`, `-2` and so on, in order.
///
/// Cost: one pass over the lines, and one `String` per heading.
#[must_use]
pub fn heading_slugs(text: &str) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut fence: Option<(char, usize)> = None;
    // The previous line, when it can be the text of a setext heading.
    let mut paragraph: Option<&str> = None;
    for (index, line) in text.lines().enumerate() {
        let line_no = u32::try_from(index).unwrap_or(u32::MAX);
        let indent = line.len() - line.trim_start_matches(' ').len();
        let body = if indent <= 3 { &line[indent..] } else { "" };

        if let Some((mark, len)) = fence {
            let run = body.chars().take_while(|&c| c == mark).count();
            if run >= len && body[run * mark.len_utf8()..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if let Some(mark) = body.chars().next().filter(|c| matches!(c, '`' | '~')) {
            let run = body.chars().take_while(|&c| c == mark).count();
            if run >= 3 {
                fence = Some((mark, run));
                paragraph = None;
                continue;
            }
        }

        if let Some(heading) = atx(body) {
            push_slug(heading, line_no, &mut counts, &mut out);
            paragraph = None;
            continue;
        }
        let underline = body.trim_end();
        if let Some(previous) = paragraph
            && !underline.is_empty()
            && (underline.chars().all(|c| c == '=') || underline.chars().all(|c| c == '-'))
        {
            push_slug(previous, line_no.saturating_sub(1), &mut counts, &mut out);
            paragraph = None;
            continue;
        }
        paragraph = (indent <= 3 && !line.trim().is_empty()).then(|| line.trim());
    }
    out
}

/// The text of an ATX heading line, its `#` run and any closing run removed,
/// or `None` when `body` is not one.
fn atx(body: &str) -> Option<&str> {
    let level = body.chars().take_while(|&c| c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &body[level..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim();
    let closed = rest.trim_end_matches('#');
    let text = if closed.is_empty() || closed.ends_with([' ', '\t']) {
        closed.trim_end()
    } else {
        rest
    };
    Some(text)
}

/// Adds the slug of `heading` at `line`, suffixed when it was seen before.
fn push_slug(
    heading: &str,
    line: u32,
    counts: &mut std::collections::HashMap<String, usize>,
    out: &mut Vec<(String, u32)>,
) {
    let base = slug(heading);
    let seen = counts.entry(base.clone()).or_insert(0);
    let slug = if *seen == 0 {
        base
    } else {
        format!("{base}-{seen}")
    };
    *seen += 1;
    out.push((slug, line));
}

/// GitHub's anchor slug of one heading's text.
#[must_use]
pub fn slug(heading: &str) -> String {
    heading
        .trim()
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_member_target_has_no_slash_and_no_dot() {
        assert_eq!(
            DocTarget::of("Core\\Money::fromCents()"),
            DocTarget::Member("Core\\Money::fromCents()")
        );
        assert_eq!(
            DocTarget::of(" self::$name "),
            DocTarget::Member("self::$name")
        );
    }

    #[test]
    fn a_file_target_splits_its_anchor() {
        assert_eq!(
            DocTarget::of("docs/guide.md#getting-started"),
            DocTarget::File {
                path: "docs/guide.md",
                anchor: Some("getting-started"),
            }
        );
        assert_eq!(
            DocTarget::of("schema.sql"),
            DocTarget::File {
                path: "schema.sql",
                anchor: None,
            }
        );
    }

    #[test]
    fn only_md_and_markdown_are_markdown() {
        assert!(is_markdown("docs/guide.md"));
        assert!(is_markdown("README.MARKDOWN"));
        assert!(!is_markdown("schema.sql"));
        assert!(!is_markdown("docs/.md"));
        assert!(!is_markdown("md"));
    }

    #[test]
    fn a_see_anchor_matches_a_github_heading_slug() {
        let text = "# Getting Started\n\
                    \n\
                    Some text.\n\
                    \n\
                    ## What's new? ##\n\
                    \n\
                    ```nvs\n\
                    # Not a heading\n\
                    ```\n\
                    \n\
                    Setext Title\n\
                    ============\n\
                    \n\
                    Second level\n\
                    ---\n\
                    \n\
                    ## Usage\n\
                    ## Usage\n\
                    ### Größe und `code`_x\n\
                    #not a heading\n\
                    ~~~\n\
                    ## also not\n\
                    ~~~\n";
        let slugs = heading_slugs(text);
        assert_eq!(
            slugs,
            vec![
                ("getting-started".to_owned(), 0),
                ("whats-new".to_owned(), 4),
                ("setext-title".to_owned(), 10),
                ("second-level".to_owned(), 13),
                ("usage".to_owned(), 16),
                ("usage-1".to_owned(), 17),
                ("größe-und-code_x".to_owned(), 18),
            ]
        );
    }

    #[test]
    fn a_thematic_break_after_a_blank_line_is_no_heading() {
        assert!(heading_slugs("Text\n\n---\n").is_empty());
    }
}
