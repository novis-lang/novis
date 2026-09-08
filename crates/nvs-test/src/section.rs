//! The section lexer: cutting a case file into its `--NAME--` sections.
//!
//! Both case formats are line-oriented in exactly the same way. A header is a
//! line that is exactly `--NAME--` or `--NAME argument--`, and a section's body
//! is every line after it up to the next header or the end of the file. This
//! module is the one parser for that shape, so `.nvst` and `.lspt` cannot drift
//! into two spellings of one format (`docs/decisions/0099.md` § 5).
//!
//! It knows no section *names*. Which names a format accepts, which of them
//! takes an argument and what that argument means are the format's own
//! questions, answered for `.nvst` in [`crate::case`]. What lives here is the
//! shape alone, which is why a second format reuses this module rather than
//! deriving from the first one.

/// One section as the line scanner found it, before any of it is interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The header's name, e.g. `EXPECT`.
    pub name: String,
    /// The header's argument, for the names a format lets take one.
    pub arg: Option<String>,
    /// The 1-based line the header is on.
    pub line: usize,
    /// Every line under the header, each still carrying its newline.
    pub body: String,
}

/// The one thing the lexer itself refuses: text before the first header.
///
/// It carries the line and no message, because the section a case must start
/// with has a name only a format knows — the wording is the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrayText {
    /// The 1-based line the stray text is on.
    pub line: usize,
}

/// Cuts `text` into its sections, in the order they were written.
///
/// A file with no header at all lexes to no sections rather than being
/// refused: "a case needs sections" is a claim about a format, and the caller
/// is what knows which one it is reading.
///
/// # Errors
///
/// Returns [`StrayText`] when a non-blank line comes before the first header,
/// so that a file which is not a case cannot be read as an empty one.
pub fn lex(text: &str) -> Result<Vec<Section>, StrayText> {
    let mut sections: Vec<Section> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if let Some((name, arg)) = header(line) {
            sections.push(Section {
                name: name.to_owned(),
                arg: arg.map(str::to_owned),
                line: number,
                body: String::new(),
            });
        } else if let Some(section) = sections.last_mut() {
            section.body.push_str(line);
            section.body.push('\n');
        } else if !line.trim().is_empty() {
            return Err(StrayText { line: number });
        }
    }
    Ok(sections)
}

/// Returns a header's section name and its argument, if `line` is a header.
///
/// A header is `--NAME--` or `--NAME argument--`; the name is uppercase ASCII,
/// digits, `-` and `_` alone, so a body line that merely contains dashes is
/// not read as one. `_` is in the set because `.phpt` spells one of its own
/// sections `--POST_RAW--`, and a name this cannot lex is a name no format
/// built on it can accept.
pub fn header(line: &str) -> Option<(&str, Option<&str>)> {
    let line = line.trim_end();
    let inner = line.strip_prefix("--")?.strip_suffix("--")?;
    let (name, arg) = match inner.split_once(' ') {
        Some((name, arg)) => (name, Some(arg.trim())),
        None => (inner, None),
    };
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return None;
    }
    Some((name, arg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn the_section_lexer_reads_a_header_with_an_argument() {
        // `--FILE <relative/path>--` is the shape `.lspt`'s multi-file cases
        // need, so the argument is the lexer's to report and the path rules
        // are the format's to enforce.
        assert_eq!(
            header("--FILE lib/helper.nvs--"),
            Some(("FILE", Some("lib/helper.nvs")))
        );
        assert_eq!(header("--FILE--"), Some(("FILE", None)));
        assert_eq!(
            header("--FILE   lib/helper.nvs  --"),
            Some(("FILE", Some("lib/helper.nvs")))
        );
        assert_eq!(header("echo \"--FILE--\";"), None);
        assert_eq!(header("-- not a header"), None);

        let sections =
            lex("--TEST--\nthe title\n--FILE lib/helper.nvs--\n<?nvs\n").expect("it lexes");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[1].name, "FILE");
        assert_eq!(sections[1].arg.as_deref(), Some("lib/helper.nvs"));
        assert_eq!(sections[1].line, 3);
        assert_eq!(sections[1].body, "<?nvs\n");
    }

    #[test]
    fn a_section_name_may_carry_an_underscore() {
        // `.nvst` is a superset of `.phpt`, which spells one section
        // `--POST_RAW--`, so the charset here is the whole of what makes that
        // name reachable at all — a lower-case one is still not a header.
        assert_eq!(header("--POST_RAW--"), Some(("POST_RAW", None)));
        assert_eq!(header("--post_raw--"), None);
    }

    #[test]
    fn the_section_lexer_is_reusable_by_a_second_format() {
        // `--REQUEST--` is `.lspt`'s and nothing here knows it, which is the
        // whole of the extraction: the second format reuses this module and
        // adds a roster of its own rather than a second parser.
        let doc = "--TEST--\ncompletion survives an unclosed brace\n--FILE--\n<?nvs\n$u-><|>\n--REQUEST--\ncompletion prefix=gr\n--EXPECT--\ngreet   method    (): string\n";
        let sections = lex(doc).expect("it lexes");
        let names: Vec<&str> = sections.iter().map(|seen| seen.name.as_str()).collect();
        assert_eq!(names, ["TEST", "FILE", "REQUEST", "EXPECT"]);
        assert_eq!(sections[1].body, "<?nvs\n$u-><|>\n");
        assert_eq!(sections[2].body, "completion prefix=gr\n");

        let refused =
            crate::case::parse(Path::new("t.nvst"), doc).expect_err("`.nvst` has no `--REQUEST--`");
        assert!(refused.to_string().contains("--REQUEST--"), "{refused}");
    }

    #[test]
    fn a_file_that_is_not_a_case_is_refused_at_its_first_line() {
        assert_eq!(lex("hello\n--TEST--\nt\n"), Err(StrayText { line: 1 }));
        assert_eq!(
            lex("\n\n--TEST--\nt\n")
                .expect("blank lines are not text")
                .len(),
            1
        );
        assert_eq!(lex("").expect("no sections is not an error").len(), 0);
    }
}
