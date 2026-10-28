//! The lens rows of each open document, held in place while its text does not
//! parse.
//!
//! An editor draws every line that has a lens as one extra row above that
//! line, so an answer that adds or drops a row moves every line below it. A
//! half-typed declaration does that on almost every keystroke: a missing `}`
//! takes the methods after it out of the tree, and a half-written method
//! header adds a declaration. So while a document does not parse, its rows are
//! the ones of the last answer over a text that did, and only a text that
//! parses adds or removes one.
//!
//! **A row moves with the lines around it.** The two texts are compared line
//! by line: a row above the first changed line keeps its line, a row below the
//! last changed line moves by the number of lines added or removed, and a row
//! inside the changed lines keeps its line, clamped into them.
//!
//! **A row shows what the text counts now.** Where the answer over the text
//! that does not parse has lenses on a row's line, the row shows those. Where
//! it has none, the row shows [`PENDING`] and runs nothing. A lens of that
//! answer on a line that is not a row is left out until the text parses.
//!
//! # What it spends
//!
//! Per open document, one copy of the last text that parsed and of the lenses
//! answered over it. Both are replaced by each answer over a text that parses,
//! and freed when the document closes or its lenses are turned off. Per answer
//! over a text that does not parse, one list of each text's lines.

use std::collections::HashMap;

use lsp_types::{CodeLens, Command, Position, Range, Uri};

/// The title of a row whose declaration the current text does not parse into.
pub(crate) const PENDING: &str = "…";

/// The last answer over a text that parsed, with that text.
#[derive(Debug)]
struct Clean {
    text: String,
    lenses: Vec<CodeLens>,
}

/// Every open document's [`Clean`] answer.
#[derive(Debug, Default)]
pub(crate) struct LensRows {
    clean: HashMap<Uri, Clean>,
}

impl LensRows {
    /// The lenses to send for `uri`, whose text is now `text`.
    ///
    /// When the text parses, that is `fresh`, which is kept for the next
    /// answer. When it does not, it is the kept rows, moved to `text`'s lines,
    /// showing what `fresh` has on each. A document with no kept answer gets
    /// `fresh`.
    pub(crate) fn steady(
        &mut self,
        uri: &Uri,
        text: &str,
        parses: bool,
        fresh: Vec<CodeLens>,
    ) -> Vec<CodeLens> {
        if parses {
            self.clean.insert(
                uri.clone(),
                Clean {
                    text: text.to_owned(),
                    lenses: fresh.clone(),
                },
            );
            return fresh;
        }
        match self.clean.get(uri) {
            Some(clean) => held(clean, text, fresh),
            None => fresh,
        }
    }

    /// Frees what is kept for `uri`.
    pub(crate) fn forget(&mut self, uri: &Uri) {
        self.clean.remove(uri);
    }
}

/// `clean`'s rows on `text`'s lines, each showing `fresh`'s lenses on its line
/// or one [`PENDING`] lens.
fn held(clean: &Clean, text: &str, fresh: Vec<CodeLens>) -> Vec<CodeLens> {
    let moved = Moved::between(&clean.text, text);
    let mut rows: Vec<u32> = clean
        .lenses
        .iter()
        .map(|lens| moved.line(lens.range.start.line))
        .collect();
    rows.sort_unstable();
    rows.dedup();

    let mut by_line: HashMap<u32, Vec<CodeLens>> = HashMap::new();
    for lens in fresh {
        by_line.entry(lens.range.start.line).or_default().push(lens);
    }
    rows.into_iter()
        .flat_map(|row| {
            by_line.remove(&row).unwrap_or_else(|| {
                let at = Position::new(row, 0);
                vec![CodeLens {
                    range: Range::new(at, at),
                    command: Some(Command {
                        title: PENDING.to_owned(),
                        command: String::new(),
                        arguments: None,
                    }),
                    data: None,
                }]
            })
        })
        .collect()
}

/// Where two texts differ, as whole lines: both start with the same `start`
/// lines and end with the same lines, which begin at `old_end` in the old text
/// and at `new_end` in the new one.
#[derive(Debug, PartialEq, Eq)]
struct Moved {
    start: usize,
    old_end: usize,
    new_end: usize,
    new_lines: usize,
}

impl Moved {
    fn between(old: &str, new: &str) -> Self {
        let old: Vec<&str> = old.split('\n').collect();
        let new: Vec<&str> = new.split('\n').collect();
        let start = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let end = old[start..]
            .iter()
            .rev()
            .zip(new[start..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        Self {
            start,
            old_end: old.len() - end,
            new_end: new.len() - end,
            new_lines: new.len(),
        }
    }

    /// The line in the new text that `line` of the old text is on.
    fn line(&self, line: u32) -> u32 {
        let old = usize::try_from(line).unwrap_or(usize::MAX);
        let new = if old < self.start {
            old
        } else if old >= self.old_end {
            old - self.old_end + self.new_end
        } else {
            old.min(self.new_end.saturating_sub(1)).max(self.start)
        };
        u32::try_from(new.min(self.new_lines.saturating_sub(1))).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lens(line: u32, title: &str) -> CodeLens {
        let at = Position::new(line, 4);
        CodeLens {
            range: Range::new(at, at),
            command: Some(Command {
                title: title.to_owned(),
                command: "nvs.showLocations".to_owned(),
                arguments: None,
            }),
            data: None,
        }
    }

    fn shown(lenses: &[CodeLens]) -> Vec<(u32, String)> {
        lenses
            .iter()
            .map(|lens| {
                let title = lens.command.as_ref().map_or("", |command| &command.title);
                (lens.range.start.line, title.to_owned())
            })
            .collect()
    }

    /// A row above an edit keeps its line, a row below moves by the lines
    /// added, and a row inside the changed lines stays inside them.
    #[test]
    fn a_row_moves_with_the_lines_around_it() {
        let moved = Moved::between("a\nb\nc\nd\n", "a\nb\nx\ny\nz\nd\n");
        assert_eq!(moved.line(0), 0);
        assert_eq!(moved.line(1), 1);
        assert_eq!(moved.line(2), 2);
        assert_eq!(moved.line(3), 5);

        let deleted = Moved::between("a\nb\nc\nd", "a\nd");
        assert_eq!(deleted.line(2), 1);
        assert_eq!(deleted.line(3), 1);

        let tail = Moved::between("a\nb\nc", "a");
        assert_eq!(tail.line(2), 0);
    }

    /// While the text does not parse, the rows are the last clean answer's: a
    /// lens the broken text adds on a new line is left out, and a row the
    /// broken text lost shows `…`.
    ///
    /// `h(` is not closed, so `g` is not a declaration in the broken text, and
    /// `h` is one that has no row yet.
    #[test]
    fn the_rows_stay_while_the_text_does_not_parse() {
        let uri: Uri = "file:///a.nvs".parse().expect("a URI");
        let mut rows = LensRows::default();
        let clean = "class A\n{\n    f()\n    g()\n}\n";
        rows.steady(
            &uri,
            clean,
            true,
            vec![
                lens(0, "no references"),
                lens(2, "1 reference"),
                lens(3, "2 references"),
            ],
        );

        let broken = "class A\n{\n    f()\n    h(\n\n    g()\n}\n";
        let answered = rows.steady(
            &uri,
            broken,
            false,
            vec![
                lens(0, "no references"),
                lens(2, "3 references"),
                lens(3, "no references"),
            ],
        );
        assert_eq!(
            shown(&answered),
            [
                (0, "no references".to_owned()),
                (2, "3 references".to_owned()),
                (5, PENDING.to_owned()),
            ]
        );
        assert!(
            answered[2]
                .command
                .as_ref()
                .is_some_and(|command| command.command.is_empty()),
            "a row with nothing counted runs nothing"
        );
    }

    /// A text that parses is answered as it is, rows added and removed.
    #[test]
    fn a_text_that_parses_is_answered_as_it_is() {
        let uri: Uri = "file:///a.nvs".parse().expect("a URI");
        let mut rows = LensRows::default();
        rows.steady(&uri, "a\nb\n", true, vec![lens(0, "no references")]);
        let answered = rows.steady(
            &uri,
            "a\nb\nc\n",
            true,
            vec![lens(0, "no references"), lens(2, "no references")],
        );
        assert_eq!(answered.len(), 2);
    }
}
