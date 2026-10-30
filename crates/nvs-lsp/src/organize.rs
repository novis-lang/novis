//! Organize imports: the `source.organizeImports` code action.
//!
//! `rule:ide/no-refactoring-introduces-an-alias` limits it to two things, and
//! it does both and nothing else. It deletes every `use` line the `W1026`
//! warning reports (`rule:statements/an-unused-use-is-a-warning`), with the
//! edit that warning already carries, and it sorts what is left in the order
//! `nvs fmt` gives (`rule:tooling/fmt-sorts-the-use-block`), by calling the
//! formatter's own sort.
//!
//! **The warnings are the analysis's own**, so a line this deletes is a line
//! `nvs check --fix` deletes, and a format after this action moves no import.
//! A file that does not parse is offered nothing, because the formatter's sort
//! does not run on it.
//!
//! **The answer is one edit**, from the first byte that changed to the last.
//! Deleting a line shifts every byte below it, so per-line edits would each
//! have to be measured against a text the client has not written yet.
//!
//! # What it spends
//!
//! Per request, one copy of the document without its unused lines, and one
//! parse of that copy. Both are dropped with the answer.

use nvs_diagnostics::{BytePos, PositionEncoding, SourceMap, Span, code};

use crate::document::Analysed;
use crate::position::range_at;
use crate::render::Action;

/// The kind the action is filed under, which LSP itself names.
pub const KIND: &str = "source.organizeImports";

/// What the action is called in the editor.
const TITLE: &str = "Organize imports";

/// The organize-imports action for the entry document of `analysed`, or
/// `None` where it would change nothing or the document does not parse.
#[must_use]
pub fn at(analysed: &Analysed, encoding: PositionEncoding) -> Option<Action> {
    let file = analysed.map.file(analysed.entry);
    let text = file.text();

    let mut unused: Vec<Span> = analysed
        .diags
        .iter()
        .filter(|diagnostic| diagnostic.code == Some(code::W_UNUSED_USE))
        .filter_map(|diagnostic| diagnostic.suggestions.first())
        .map(|suggestion| suggestion.span)
        .filter(|span| span.file == analysed.entry)
        .collect();
    unused.sort_by_key(|span| (span.start, span.end));

    let mut kept = String::with_capacity(text.len());
    let mut from = 0;
    for span in &unused {
        let (start, end) = (span.start as usize, span.end as usize);
        if start < from {
            continue;
        }
        kept.push_str(&text[from..start]);
        from = end;
    }
    kept.push_str(&text[from..]);

    let mut map = SourceMap::new();
    let id = map.add(file.name(), kept);
    let organized = match nvs_fmt::sorted_imports(map.file(id)) {
        Ok(Some(sorted)) => sorted,
        Ok(None) => map.file(id).text().to_owned(),
        Err(_) => return None,
    };
    if organized == text {
        return None;
    }

    let (start, end) = changed(text, &organized);
    let to = |at: usize| BytePos::try_from(at).unwrap_or(BytePos::MAX);
    Some(Action {
        title: TITLE.to_owned(),
        kind: KIND.to_owned(),
        range: range_at(
            file,
            Span::new(analysed.entry, to(start), to(end)),
            encoding,
        ),
        replacement: organized[start..organized.len() - (text.len() - end)].to_owned(),
        also: Vec::new(),
    })
}

/// The bytes of `old` that differ from `new`: where the common start ends,
/// and where the common end begins. Both are on a character boundary.
pub(crate) fn changed(old: &str, new: &str) -> (usize, usize) {
    let mut start = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(left, right)| left == right)
        .count();
    while !old.is_char_boundary(start) {
        start -= 1;
    }
    let room = old.len().min(new.len()) - start;
    let mut same = old
        .bytes()
        .rev()
        .zip(new.bytes().rev())
        .take(room)
        .take_while(|(left, right)| left == right)
        .count();
    while !old.is_char_boundary(old.len() - same) || !new.is_char_boundary(new.len() - same) {
        same -= 1;
    }
    (start, old.len() - same)
}

#[cfg(test)]
mod tests {
    use super::changed;

    #[test]
    fn the_changed_bytes_are_between_the_common_start_and_end() {
        let old = "use B;\nuse A;\nclass C {}\n";
        let new = "use A;\nuse B;\nclass C {}\n";
        assert_eq!(changed(old, new), (4, 12));

        // A deletion: the common end does not run into the common start.
        assert_eq!(changed("aXa", "aa"), (1, 2));
    }
}
