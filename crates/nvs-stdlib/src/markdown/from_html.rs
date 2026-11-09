//! `Core\Markdown::fromHtml` — `rule:core-classes/markdown-from-html`'s
//! converter from HTML to Markdown.
//!
//! # Three steps, and only the sanitizer's holds a policy
//!
//! The document is parsed by [`crate::html::parse`], or read out of a node a
//! program already holds ([`crate::xml::parsed_of`]). [`rebuilt`] then keeps
//! what `rule:core-classes/html-sanitize`'s list keeps, so a `<script>`'s text,
//! an attribute the list does not grant and a `javascript:` URL are gone before
//! the walk here reads anything. The walk writes Markdown from what is left.
//!
//! # Text never becomes Markdown syntax
//!
//! Every text node is escaped by [`Inline::flush`]: the characters that are
//! syntax anywhere always get a backslash, and the ones that are syntax only
//! at the start of a line get one there. GFM's extended autolinks are found in
//! a paragraph's text after escapes are read, so no backslash can stop one.
//! A text that would autolink is cut with an empty HTML comment, `<!-- -->`,
//! before the `.` of `www.`, the `:` of `://` and the `@` of an address. The
//! renderer reads text on each side of the comment alone, and the sanitizer
//! drops the comment.
//!
//! # What Markdown cannot write
//!
//! A table Markdown cannot hold, a definition list, a figure and a `pre`
//! with a carriage return are written as their sanitized HTML on one line,
//! which CommonMark reads as one HTML
//! block. A line feed inside it is written `&#10;`, so the block has no blank
//! line to end it early. An inline element Markdown has no syntax for, such
//! as `<sup>`, is written as its start tag, its content as Markdown, and its
//! end tag. Emphasis whose content starts or ends with punctuation is written
//! as its tags too, because `*` there is not read as emphasis. So is emphasis
//! whose content has its own delimiter between two letters, as `<em>` in
//! `<em>a<em>b</em>c</em>`: `*a*b*c*` reads as two emphases side by side.
//!
//! # What this spends
//!
//! Per call, per `rule:programs/memory-priority`: the parse's tree, the
//! rebuilt tree, and the Markdown, all attributed to the request. Each
//! container's Markdown is written into a string of its own and then copied
//! into its parent's, so an element nested `d` deep is copied `d` times.
//! [`DEPTH`] bounds `d`, which keeps the work linear and the native stack
//! small: past it, an element's text is written and its markup is not.
//!
//! The tree is held whole, not written one top-level subtree at a time: the
//! tree builder still changes a node after a later one opens (text fostered
//! in front of a table joins the text before it), and every input is already
//! whole in memory, so streaming would save only the tree
//! (`rule:core-classes/markdown-from-html`'s *Large documents*).

use std::cell::Cell;
use std::rc::Rc;

use nvs_runtime::Fault;

use crate::html::{rebuilt, settled, source};
use crate::xml::{Kind, Parsed};

/// The member every fault names.
pub(super) const MEMBER: &str = r"Core\Markdown::fromHtml";

/// How deeply the walk follows elements before it writes the text under one
/// as plain text. Each level is a native frame and a copy of its Markdown,
/// so this bounds both. No document a person writes nests this deep.
const DEPTH: usize = 128;

/// The empty comment that ends a run of text the autolink scan reads.
const CUT: &str = "<!-- -->";

#[cfg(test)]
thread_local! {
    /// The bytes the walk has read and copied on this thread: the work the
    /// tests count, as the engine's steps are for `toHtml`.
    pub(super) static WORK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Adds `bytes` to [`WORK`]. Outside tests it does nothing.
fn work(bytes: usize) {
    #[cfg(test)]
    WORK.with(|cell| cell.set(cell.get() + bytes as u64));
    #[cfg(not(test))]
    let _ = bytes;
}

/// `tree`, a parse's document or a node, as Markdown.
///
/// # Errors
///
/// The memory limit's `FATAL`.
pub(super) fn markdown(tree: Parsed) -> Result<String, Fault> {
    let tree = if tree.kind == Kind::Document {
        tree
    } else {
        let mut document = Parsed::new(Kind::Document);
        document.children.push(tree);
        document
    };
    let kept = rebuilt(&tree);
    drop(tree);
    let nodes: Vec<&Parsed> = kept.children.iter().collect();
    let notes = match footnotes(&nodes) {
        Some((rule, items)) => written(&nodes[..rule], &items)?,
        None => None,
    };
    let mut out = match notes {
        Some(out) => out,
        None => written(&nodes, &[])?.unwrap_or_default(),
    };
    // A Markdown reader drops a byte-order mark at the start of the document,
    // so text that starts with U+FEFF writes it as an entity.
    if out.starts_with('\u{FEFF}') {
        out.replace_range(..'\u{FEFF}'.len_utf8(), "&#xFEFF;");
    }
    if !out.is_empty() {
        out.push('\n');
    }
    Ok(out)
}

/// `nodes`, a document's top-level nodes, as Markdown, with `notes`, the items
/// of a footnote section, as footnote definitions after them. `None` when the
/// footnote references written do not first appear in the order 1, 2, 3 and
/// so on up to the last note: the renderer numbers notes by their first
/// reference, so it would number these another way.
fn written(nodes: &[&Parsed], notes: &[&Parsed]) -> Result<Option<String>, Fault> {
    let seen = Rc::new(Cell::new(Notes {
        count: notes.len(),
        seen: 0,
        broken: false,
    }));
    let mut writer = Writer {
        task: false,
        notes: (!notes.is_empty()).then(|| Rc::clone(&seen)),
    };
    let mut parts = Vec::new();
    writer.collect(nodes, 0, &mut parts)?;
    for (at, item) in notes.iter().enumerate() {
        writer.note(item, at + 1, &mut parts)?;
    }
    let seen = seen.get();
    if seen.broken || seen.seen != seen.count {
        return Ok(None);
    }
    joined(&parts, false).map(Some)
}

/// The footnote references written so far, while a document with a footnote
/// section is written.
#[derive(Clone, Copy)]
struct Notes {
    /// How many notes the section has.
    count: usize,
    /// How many notes a reference has been written for.
    seen: usize,
    /// Whether a reference came before the references to the notes numbered
    /// below it.
    broken: bool,
}

/// The footnote section the renderer writes at the end of `nodes`: the index
/// of its `<hr>`, and the items of the `<ol>` after it. Item `n` has the id
/// `md-fn-n` and ends with its link back to the reference, `↩`.
fn footnotes<'a>(nodes: &[&'a Parsed]) -> Option<(usize, Vec<&'a Parsed>)> {
    let mut last = nodes
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, node)| !blank(node));
    let (_, list) = last.next()?;
    let (rule, hr) = last.next()?;
    let bare = |node: &Parsed, name: &str| {
        node.kind == Kind::Element && node.name == name && node.attributes.is_empty()
    };
    if !bare(hr, "hr") || !bare(list, "ol") {
        return None;
    }
    let items: Vec<&Parsed> = list.children.iter().filter(|node| !blank(node)).collect();
    if items.is_empty() {
        return None;
    }
    for (at, item) in items.iter().enumerate() {
        let number = at + 1;
        let id = format!("md-fn-{number}");
        let named = item.kind == Kind::Element
            && item.name == "li"
            && item.attributes.len() == 1
            && attribute(item, "id") == Some(id.as_str());
        if !named || back_link(item, number).is_none() {
            return None;
        }
    }
    Some((rule, items))
}

/// Where the link back to note `number`'s reference is in `item`: the index
/// of the last child that is not white space, and, when that child is a
/// paragraph, the index of the link in it.
fn back_link(item: &Parsed, number: usize) -> Option<(usize, Option<usize>)> {
    let href = format!("#md-fnref-{number}");
    let is_back = |node: &Parsed| {
        node.kind == Kind::Element
            && node.name == "a"
            && node.attributes.len() == 1
            && attribute(node, "href") == Some(href.as_str())
            && matches!(node.children.as_slice(), [arrow] if arrow.kind == Kind::Text && arrow.text == "\u{21A9}")
    };
    let end = item.children.iter().rposition(|node| !blank(node))?;
    let last = &item.children[end];
    if is_back(last) {
        return Some((end, None));
    }
    if last.kind == Kind::Element && last.name == "p" {
        let inner = last.children.iter().rposition(|node| !blank(node))?;
        if is_back(&last.children[inner]) {
            return Some((end, Some(inner)));
        }
    }
    None
}

/// What one block of Markdown is, which decides what may follow it on the
/// next line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Paragraph,
    Heading,
    Fence,
    Rule,
    /// A quote. `open` is whether it ends in a block that the next line could
    /// continue as a lazy line. `shut` is whether a last `>` line ends that
    /// block and keeps a list around the quote tight: it does after a
    /// paragraph, a table or such a quote, and not after a list.
    Quote {
        open: bool,
        shut: bool,
    },
    /// A list, with its marker character: `-`, `*` or `+`, `.` or `)`. `first` is
    /// whether an ordered list starts at 1, the one ordered list that may
    /// interrupt a paragraph. `empty` is whether its first item is empty: a
    /// bare marker on the line after a paragraph continues the paragraph, and
    /// a bare `-` makes it a setext heading. `open` is whether its last item
    /// ends in a block that the next line could continue.
    List {
        marker: char,
        first: bool,
        empty: bool,
        open: bool,
    },
    Table,
    Html,
}

impl Part {
    /// Whether this block may start on the line after a paragraph.
    fn interrupts(self) -> bool {
        match self {
            Self::Heading | Self::Fence | Self::Rule | Self::Quote { .. } => true,
            Self::List {
                marker,
                first,
                empty,
                ..
            } => !empty && (matches!(marker, '-' | '*' | '+') || first),
            Self::Paragraph | Self::Table | Self::Html => false,
        }
    }

    /// Whether the next line could continue this block: a paragraph's lazy
    /// line, a table's row or an HTML block's line, or one of them at the end
    /// of a quote or a list.
    fn open(self) -> bool {
        match self {
            Self::Paragraph | Self::Table | Self::Html => true,
            Self::Quote { open, .. } | Self::List { open, .. } => open,
            Self::Heading | Self::Fence | Self::Rule => false,
        }
    }
}

/// `parts` joined into one container's Markdown. A loose container puts a
/// blank line between every two blocks. A tight one, a list item of a tight
/// list, puts a line feed where the second block cannot be read as part of
/// the first, and a blank line elsewhere ([`tight_gap`]).
fn joined(parts: &[(Part, String)], tight: bool) -> Result<String, Fault> {
    let total = parts.iter().map(|(_, text)| text.len() + 4).sum::<usize>();
    work(total + parts.len());
    nvs_runtime::affordable(Some(total), MEMBER)?;
    let mut out = String::with_capacity(total);
    let mut previous: Option<Part> = None;
    for (part, text) in parts {
        if let Some(before) = previous {
            let gap = tight.then(|| tight_gap(before, *part)).flatten();
            out.push_str(gap.unwrap_or("\n\n"));
        }
        out.push_str(text);
        previous = Some(*part);
    }
    Ok(out)
}

/// What goes between `before` and `part` in a tight container, or `None`
/// where only a blank line can, which makes a list around them loose. A line
/// feed goes where `part` cannot be read as part of `before`
/// ([`on_next_line`]). An open quote that a `>` line can shut gets that line
/// first, where `part` may follow a closed quote.
fn tight_gap(before: Part, part: Part) -> Option<&'static str> {
    if on_next_line(before, part) {
        return Some("\n");
    }
    let closed = Part::Quote {
        open: false,
        shut: false,
    };
    match before {
        Part::Quote {
            open: true,
            shut: true,
        } if on_next_line(closed, part) => Some("\n>\n"),
        _ => None,
    }
}

/// Whether `part` may be written on the line right after `before` in a tight
/// container, where it cannot be read as part of `before`. After a paragraph,
/// that is a block that interrupts a paragraph. After a quote or list that
/// ends in an open block, it is the same, except that any list marker ends
/// that block: an empty item or a number other than 1 may interrupt a
/// paragraph that is not in the item's own container. A quote after a quote
/// joins it, and a list after a list must have another marker. After a
/// closed quote or list, a block that does not start with `>` ends the
/// quote, and one outside the list's indent ends the list: a list there has
/// another marker.
fn on_next_line(before: Part, part: Part) -> bool {
    match (before, part) {
        (Part::Heading | Part::Fence | Part::Rule, _) => true,
        (Part::Paragraph, _) => part.interrupts(),
        (Part::Quote { open: true, .. }, Part::Quote { .. }) => false,
        (Part::Quote { open: true, .. }, Part::List { .. }) => true,
        (
            Part::List {
                open: true, marker, ..
            },
            Part::List { marker: next, .. },
        ) => marker != next,
        (Part::Quote { open: true, .. } | Part::List { open: true, .. }, _) => part.interrupts(),
        (Part::Quote { open: false, .. }, _) => !matches!(part, Part::Quote { .. }),
        (Part::List { open: false, .. }, _) => true,
        (Part::Table | Part::Html, _) => false,
    }
}

/// The elements the walk reads as blocks. Any other element is inline.
fn is_block(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "blockquote"
            | "pre"
            | "ul"
            | "ol"
            | "li"
            | "hr"
            | "table"
            | "caption"
            | "colgroup"
            | "col"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "td"
            | "th"
            | "dl"
            | "dt"
            | "dd"
            | "figure"
            | "figcaption"
    )
}

/// An attribute's value, or `None` when `node` has no such attribute.
fn attribute<'a>(node: &'a Parsed, name: &str) -> Option<&'a str> {
    node.attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

/// Whether `ch` is white space CSS collapses: a space, a tab or a line break.
/// A form feed is HTML whitespace but not CSS white space; a browser shows it
/// as a character, and Markdown keeps it in a paragraph, so it is text here.
fn css_space(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r')
}

/// Whether `node` is a text node of CSS white space alone.
fn blank(node: &Parsed) -> bool {
    node.kind == Kind::Text && node.text.chars().all(css_space)
}

/// The walk's state across blocks.
struct Writer {
    /// Whether the next paragraph is the first of a list item, where a
    /// checkbox is written as a task marker.
    task: bool,
    /// The references written, when the document has a footnote section.
    notes: Option<Rc<Cell<Notes>>>,
}

impl Writer {
    /// `nodes`, one container's children, as blocks pushed onto `parts`.
    /// Inline nodes between two blocks become one paragraph.
    fn collect(
        &mut self,
        nodes: &[&Parsed],
        depth: usize,
        parts: &mut Vec<(Part, String)>,
    ) -> Result<(), Fault> {
        let mut run: Vec<&Parsed> = Vec::new();
        for node in nodes {
            // A loose task item's box is written in front of its first
            // paragraph, as cmark-gfm writes it. The box starts that paragraph.
            let boxed = self.task
                && node.kind == Kind::Element
                && node.name == "p"
                && depth < DEPTH
                && run.iter().any(|node| node.name == "input")
                && run.iter().all(|node| {
                    blank(node) || (node.kind == Kind::Element && node.name == "input")
                });
            if boxed {
                run.extend(node.children.iter());
                self.paragraph(&run, depth + 1, parts);
                run.clear();
            } else if node.kind == Kind::Element && is_block(&node.name) {
                self.paragraph(&run, depth, parts);
                run.clear();
                self.block(node, depth + 1, parts)?;
            } else {
                run.push(node);
            }
        }
        self.paragraph(&run, depth, parts);
        Ok(())
    }

    /// Pushes `part` and ends a list item's first block.
    fn push(&mut self, parts: &mut Vec<(Part, String)>, part: Part, text: String) {
        self.task = false;
        parts.push((part, text));
    }

    /// `nodes`, inline content, as one paragraph, or nothing when they hold
    /// no text.
    fn paragraph(&mut self, nodes: &[&Parsed], depth: usize, parts: &mut Vec<(Part, String)>) {
        if nodes.iter().all(|node| blank(node)) {
            return;
        }
        let mut inline = Inline::new(Place::Paragraph, false);
        inline.task = self.task;
        inline.notes.clone_from(&self.notes);
        inline.nodes(nodes.iter().copied(), depth);
        let text = inline.whole();
        if !text.is_empty() {
            self.push(parts, Part::Paragraph, text);
        }
    }

    /// One block element pushed onto `parts`.
    fn block(
        &mut self,
        node: &Parsed,
        depth: usize,
        parts: &mut Vec<(Part, String)>,
    ) -> Result<(), Fault> {
        if depth > DEPTH {
            let mut inline = Inline::new(Place::Paragraph, false);
            inline.text(&text_content(node, " "));
            let piece = inline.finish();
            if !piece.text.is_empty() {
                self.push(parts, Part::Paragraph, piece.text);
            }
            return Ok(());
        }
        let children: Vec<&Parsed> = node.children.iter().collect();
        match node.name.as_str() {
            "p" => self.paragraph(&children, depth, parts),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = usize::from(node.name.as_bytes()[1] - b'0');
                let mut inline = Inline::new(Place::Heading, false);
                inline.notes.clone_from(&self.notes);
                inline.nodes(children.iter().copied(), depth);
                let inner = inline.whole();
                let mut text = "#".repeat(level);
                if !inner.is_empty() {
                    text.push(' ');
                    text.push_str(&inner);
                }
                self.push(parts, Part::Heading, text);
            }
            "blockquote" => {
                let mut inner = Vec::new();
                self.collect(&children, depth, &mut inner)?;
                let last = inner.last().map(|(part, _)| *part);
                let open = last.is_some_and(Part::open);
                let shut = matches!(
                    last,
                    Some(Part::Paragraph | Part::Table | Part::Quote { shut: true, .. })
                );
                let text = prefixed(&joined(&inner, false)?, ">", ">");
                self.push(parts, Part::Quote { open, shut }, text);
            }
            "pre" => match fenced(node) {
                Some(text) => self.push(parts, Part::Fence, text),
                None => self.push(parts, Part::Html, html_block(node)?),
            },
            "ul" | "ol" => self.list(node, depth, parts)?,
            "hr" => self.push(parts, Part::Rule, "***".to_owned()),
            "table" => match gfm_table(node, depth, self.notes.as_ref())? {
                Some(text) => self.push(parts, Part::Table, text),
                None => self.push(parts, Part::Html, html_block(node)?),
            },
            "dl" | "figure" => self.push(parts, Part::Html, html_block(node)?),
            // `div`, and an element of a table or a list found outside one:
            // its children are blocks of the container it is in.
            _ => self.collect(&children, depth, parts)?,
        }
        Ok(())
    }

    /// A `ul` or an `ol` pushed onto `parts`.
    ///
    /// A list right after another list of the same kind changes its marker,
    /// `-` to `*` or `.` to `)`, because Markdown joins two lists with the
    /// same marker into one. A list whose item would start with a line such
    /// as `- - -` or `* ***`, a thematic break, uses another marker.
    /// A child that is not an `li`, such as a list
    /// written straight inside a list, goes into the item before it.
    fn list(
        &mut self,
        node: &Parsed,
        depth: usize,
        parts: &mut Vec<(Part, String)>,
    ) -> Result<(), Fault> {
        let ordered = node.name == "ol";
        let start = if ordered {
            match attribute(node, "start").map(str::trim) {
                None => 1,
                Some(written) => match written.parse::<i64>() {
                    Ok(start) if (0..=999_999_999).contains(&start) => start,
                    // CommonMark numbers a list with nine digits at most.
                    Ok(_) => {
                        self.push(parts, Part::Html, html_block(node)?);
                        return Ok(());
                    }
                    Err(_) => 1,
                },
            }
        } else {
            0
        };
        let after = parts.last().map(|(part, _)| *part);
        let mut marker = match (ordered, after) {
            (false, Some(Part::List { marker: '-', .. })) => '*',
            (false, _) => '-',
            (true, Some(Part::List { marker: '.', .. })) => ')',
            (true, _) => '.',
        };

        let mut items: Vec<Vec<&Parsed>> = Vec::new();
        for child in &node.children {
            if child.kind == Kind::Element && child.name == "li" {
                items.push(child.children.iter().collect());
            } else if blank(child) {
            } else if let Some(last) = items.last_mut() {
                last.push(child);
            } else {
                items.push(vec![child]);
            }
        }
        if items.is_empty() {
            return Ok(());
        }
        let mut loose = items.iter().any(|item| {
            item.iter()
                .any(|child| child.kind == Kind::Element && child.name == "p")
        });

        let mut blocks = Vec::with_capacity(items.len());
        for item in &items {
            let mut inner = Vec::new();
            self.task = true;
            self.collect(item, depth, &mut inner)?;
            self.task = false;
            blocks.push(inner);
        }
        // Two blocks of an item that cannot share a line get a blank line
        // between them, and that blank line makes the whole list loose. The
        // list is written loose, so it reads back the same.
        loose = loose
            || blocks.iter().any(|inner| {
                inner
                    .windows(2)
                    .any(|pair| tight_gap(pair[0].0, pair[1].0).is_none())
            });
        let open = blocks
            .last()
            .and_then(|inner| inner.last())
            .is_some_and(|(part, _)| part.open());
        let mut contents = Vec::with_capacity(blocks.len());
        for inner in &blocks {
            contents.push(joined(inner, !loose)?);
        }
        let empty = contents.first().is_some_and(String::is_empty);
        // An item that starts with a list or a rule writes its first line
        // after the marker, and `- - -` or `* ***` is a thematic break. A `+`
        // never makes one. The marker is the first of `-`, `*` and `+` that
        // makes none and differs from the list before.
        if !ordered {
            let breaks = |marker: char| {
                contents.iter().any(|content| {
                    thematic_break(&format!(
                        "{marker} {}",
                        content.lines().next().unwrap_or("")
                    ))
                })
            };
            if breaks(marker) {
                let before = match after {
                    Some(Part::List { marker, .. }) => Some(marker),
                    _ => None,
                };
                let others = ['-', '*', '+'].into_iter().filter(|&m| Some(m) != before);
                marker = others
                    .clone()
                    .find(|&m| !breaks(m))
                    .or_else(|| others.clone().next_back())
                    .unwrap_or('+');
            }
        }

        let mut written = Vec::with_capacity(items.len());
        for (number, content) in (start..).zip(contents) {
            // Only the first number is read, so a later one past nine digits
            // repeats the first.
            let number = if number > 999_999_999 { start } else { number };
            let mark = if ordered {
                format!("{number}{marker}")
            } else {
                marker.to_string()
            };
            let first = format!("{mark} ");
            let rest = " ".repeat(first.len());
            written.push(if content.is_empty() {
                mark
            } else {
                prefixed(&content, &first, &rest)
            });
        }
        let text = written.join(if loose { "\n\n" } else { "\n" });
        self.push(
            parts,
            Part::List {
                marker,
                first: start == 1,
                empty,
                open,
            },
            text,
        );
        Ok(())
    }

    /// Item `number` of a footnote section, which [`footnotes`] found,
    /// pushed onto `parts` as the note's definition `[^n]:`. Its link back to
    /// the reference is not written, since the renderer writes it again.
    fn note(
        &mut self,
        item: &Parsed,
        number: usize,
        parts: &mut Vec<(Part, String)>,
    ) -> Result<(), Fault> {
        let Some((end, inner)) = back_link(item, number) else {
            return Ok(());
        };
        let children: Vec<&Parsed> = item.children.iter().collect();
        let mut blocks = Vec::new();
        self.collect(&children[..end], 2, &mut blocks)?;
        if let Some(inner) = inner {
            let last: Vec<&Parsed> = children[end].children[..inner].iter().collect();
            self.paragraph(&last, 3, &mut blocks);
        }
        let content = joined(&blocks, false)?;
        let label = format!("[^{number}]:");
        let text = if content.is_empty() {
            label
        } else {
            prefixed(&content, &format!("{label} "), "    ")
        };
        self.push(parts, Part::Paragraph, text);
        Ok(())
    }
}

/// `node` as a GFM table, or `None` when a GFM table cannot hold it.
///
/// A GFM table has one header row of `th` cells, the same number of cells in
/// every row, one alignment per column, and inline content in each cell. A
/// `caption`, a `tfoot`, a `colspan` or a `rowspan` has no place in it.
/// `notes` is the footnote references written, as [`Writer`] has them.
fn gfm_table(
    node: &Parsed,
    depth: usize,
    notes: Option<&Rc<Cell<Notes>>>,
) -> Result<Option<String>, Fault> {
    let mut header: Option<&Parsed> = None;
    let mut body: Vec<&Parsed> = Vec::new();
    for child in &node.children {
        if blank(child) {
            continue;
        }
        if child.kind != Kind::Element {
            return Ok(None);
        }
        match child.name.as_str() {
            "thead" => {
                let rows: Vec<&Parsed> = child.children.iter().filter(|row| !blank(row)).collect();
                if header.is_some() || !body.is_empty() || rows.len() != 1 {
                    return Ok(None);
                }
                header = Some(rows[0]);
            }
            "tbody" => {
                for row in &child.children {
                    if !blank(row) {
                        body.push(row);
                    }
                }
            }
            "tr" => body.push(child),
            "colgroup" => {}
            _ => return Ok(None),
        }
    }
    let header = match header {
        Some(row) => row,
        None if !body.is_empty() => body.remove(0),
        None => return Ok(None),
    };
    let Some(heads) = cells(header, "th") else {
        return Ok(None);
    };
    if heads.is_empty() {
        return Ok(None);
    }
    let aligns: Vec<Option<&str>> = heads.iter().map(|cell| attribute(cell, "align")).collect();
    let mut rows = Vec::with_capacity(body.len());
    for row in &body {
        let Some(row) = cells(row, "td") else {
            return Ok(None);
        };
        if row.len() != heads.len()
            || row
                .iter()
                .zip(&aligns)
                .any(|(cell, align)| attribute(cell, "align") != *align)
        {
            return Ok(None);
        }
        rows.push(row);
    }
    if heads
        .iter()
        .chain(rows.iter().flatten())
        .any(|cell| !gfm_cell(cell))
    {
        return Ok(None);
    }

    let line = |cells: &[&Parsed]| {
        let mut out = String::from("|");
        for cell in cells {
            let mut inline = Inline::new(Place::Cell, false);
            inline.notes = notes.cloned();
            inline.nodes(cell.children.iter(), depth);
            out.push(' ');
            out.push_str(&inline.whole());
            out.push_str(" |");
        }
        out
    };
    let mut lines = vec![line(&heads)];
    let mut rule = String::from("|");
    for align in &aligns {
        rule.push_str(match *align {
            Some("left") => " :--- |",
            Some("center") => " :---: |",
            Some("right") => " ---: |",
            _ => " --- |",
        });
    }
    lines.push(rule);
    for row in &rows {
        lines.push(line(row));
    }
    let total = lines.iter().map(|line| line.len() + 1).sum::<usize>();
    nvs_runtime::affordable(Some(total), MEMBER)?;
    Ok(Some(lines.join("\n")))
}

/// `row`'s cells when every one is a `name` element, or `None`.
fn cells<'a>(row: &'a Parsed, name: &str) -> Option<Vec<&'a Parsed>> {
    if row.kind != Kind::Element || row.name != "tr" {
        return None;
    }
    let mut out = Vec::new();
    for cell in &row.children {
        if blank(cell) {
            continue;
        }
        if cell.kind != Kind::Element || cell.name != name {
            return None;
        }
        out.push(cell);
    }
    Some(out)
}

/// Whether a GFM table cell can hold `cell`: no `colspan` or `rowspan`, no
/// block element under it, and no `|` inside a code span, where GFM's escape
/// for it is not read the same way by every renderer.
fn gfm_cell(cell: &Parsed) -> bool {
    if attribute(cell, "colspan").is_some() || attribute(cell, "rowspan").is_some() {
        return false;
    }
    let mut stack: Vec<(&Parsed, bool)> =
        cell.children.iter().map(|child| (child, false)).collect();
    while let Some((node, in_code)) = stack.pop() {
        match node.kind {
            Kind::Text if in_code && node.text.contains('|') => return false,
            Kind::Element if is_block(&node.name) => return false,
            Kind::Element => {
                let code = in_code || node.name == "code";
                stack.extend(node.children.iter().map(|child| (child, code)));
            }
            _ => {}
        }
    }
    true
}

/// `text` with `first` before its first line and `rest` before every other
/// line that is not empty. An empty line gets `rest` trimmed, so a quote's
/// blank line is `>` and a list item's is empty.
fn prefixed(text: &str, first: &str, rest: &str) -> String {
    work(text.len());
    let mut out = String::with_capacity(text.len() + first.len());
    for (at, line) in text.split('\n').enumerate() {
        if at > 0 {
            out.push('\n');
        }
        let prefix = if at == 0 { first } else { rest };
        if line.is_empty() {
            out.push_str(prefix.trim_end());
        } else {
            out.push_str(prefix);
            if prefix.ends_with('>') {
                out.push(' ');
            }
            out.push_str(line);
        }
    }
    out
}

/// A `pre` as a fenced code block. The fence is one backtick longer than the
/// longest run of backticks in the code, and three at least. A `code` inside
/// the `pre` with a `language-` class gives the fence its language. `None`
/// when the code has a carriage return, which Markdown reads as the end of a
/// line: such a `pre` is written as its HTML, which keeps it as `&#13;`.
fn fenced(node: &Parsed) -> Option<String> {
    let code = pre_text(node);
    if code.contains('\r') {
        return None;
    }
    let mut longest = 0;
    let mut run = 0;
    for ch in code.chars() {
        if ch == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    let fence = "`".repeat((longest + 1).max(3));
    let mut elements = node.children.iter().filter(|child| !blank(child));
    let language = match (elements.next(), elements.next()) {
        (Some(only), None) if only.kind == Kind::Element && only.name == "code" => {
            attribute(only, "class").and_then(|class| class.strip_prefix("language-"))
        }
        _ => None,
    };
    let mut out = String::with_capacity(code.len() + 2 * fence.len() + 16);
    out.push_str(&fence);
    out.push_str(language.unwrap_or(""));
    out.push('\n');
    out.push_str(&code);
    if !code.is_empty() && !code.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&fence);
    Some(out)
}

/// What a `pre` shows: its text as written, a line feed for each `br`, and
/// a line feed around each block element inside it.
fn pre_text(node: &Parsed) -> String {
    enum Step<'a> {
        Node(&'a Parsed),
        Leave,
    }
    let mut out = String::new();
    let mut stack: Vec<Step<'_>> = node.children.iter().rev().map(Step::Node).collect();
    while let Some(step) = stack.pop() {
        match step {
            Step::Leave => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Step::Node(node) => match node.kind {
                Kind::Text => {
                    work(node.text.len());
                    out.push_str(&node.text);
                }
                Kind::Element if node.name == "br" => out.push('\n'),
                Kind::Element => {
                    let block = is_block(&node.name);
                    if block {
                        if !out.is_empty() && !out.ends_with('\n') {
                            out.push('\n');
                        }
                        stack.push(Step::Leave);
                    }
                    stack.extend(node.children.iter().rev().map(Step::Node));
                }
                _ => {}
            },
        }
    }
    out
}

/// Every text under `node`, in order, with `br` written as `line`.
fn text_content(node: &Parsed, line: &str) -> String {
    let mut out = String::new();
    let mut stack: Vec<&Parsed> = node.children.iter().rev().collect();
    if node.kind == Kind::Text {
        out.push_str(&node.text);
    }
    while let Some(node) = stack.pop() {
        work(1);
        match node.kind {
            Kind::Text => {
                work(node.text.len());
                out.push_str(&node.text);
            }
            Kind::Element if node.name == "br" => out.push_str(line),
            Kind::Element => stack.extend(node.children.iter().rev()),
            _ => {}
        }
    }
    out
}

/// `node`'s start tag as the sanitizer writes it, and its end tag, or `None`
/// for an element with no end tag. A line feed in an attribute is written
/// `&#10;`, so the tag stays on its line.
fn tags(node: &Parsed) -> (String, Option<String>) {
    let mut shell = Parsed::new(Kind::Element);
    shell.name.push_str(&node.name);
    shell.attributes.clone_from(&node.attributes);
    let mut document = Parsed::new(Kind::Document);
    document.children.push(shell);
    let written = one_line(&source(&document));
    let end = format!("</{}>", node.name);
    match written.strip_suffix(&end) {
        Some(open) => (open.to_owned(), Some(end)),
        None => (written, None),
    }
}

/// `node` as sanitized HTML on one line: CommonMark reads it as one HTML
/// block, which ends only at a blank line.
///
/// The HTML is the sanitizer's answer for the node's source, which the
/// renderer writes unchanged. The input was parsed in quirks mode, where
/// `<table>` does not close an open `<p>`, and the sanitizer parses in
/// no-quirks mode, so a `p` around a `table` written as it was would come
/// back as two paragraphs and a table. The cost is a second parse of each
/// HTML block, which only a construct Markdown cannot write pays.
///
/// # Errors
///
/// The memory limit's `FATAL`.
fn html_block(node: &Parsed) -> Result<String, Fault> {
    let (open, end) = tags(node);
    let mut out = open;
    out.push_str(&source(node));
    if let Some(end) = end {
        out.push_str(&end);
    }
    let out = one_line(&settled(&out, None, MEMBER)?);
    work(out.len());
    Ok(out)
}

/// `html` with each line feed and carriage return written as a character
/// reference, which HTML reads as the same character.
fn one_line(html: &str) -> String {
    if !html.contains(['\n', '\r']) {
        return html.to_owned();
    }
    let mut out = String::with_capacity(html.len() + 16);
    for ch in html.chars() {
        match ch {
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Where inline content is written, which decides how a line break is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    /// A paragraph, where a break is a backslash at the end of the line.
    Paragraph,
    /// A heading, which is one line, so a break is `<br />`.
    Heading,
    /// A table cell, which is one line, so a break is `<br />`.
    Cell,
}

/// The Markdown of one element's inline content, and the whitespace and the
/// number of breaks at its two ends. The parent writes those outside the
/// element's own syntax, where Markdown reads them.
struct Piece {
    text: String,
    lead: bool,
    lead_break: usize,
    trail: bool,
    trail_break: usize,
}

/// Inline content being written.
///
/// Whitespace collapses as CSS collapses it in an inline context: a run of
/// spaces, tabs and line feeds is one space, and none is written at the start
/// or the end. A space and the breaks are held until the next text arrives,
/// so neither ends up at the end of a line. A browser shows each break, so
/// each one is written; breaks with no text before or after them in the
/// block are written as `<br />`, since Markdown has no syntax for them.
struct Inline {
    place: Place,
    out: String,
    /// Text not yet escaped. It is escaped as one run, so the autolink cut
    /// sees the text the renderer will see.
    run: String,
    space: bool,
    /// The breaks held until text follows them.
    brk: usize,
    lead: bool,
    /// The breaks before the first text.
    lead_break: usize,
    /// Inside a link's text, where the renderer reads no autolink.
    link: bool,
    /// Whether a checkbox here is a task list item's marker.
    task: bool,
    /// The footnote references written, when the document has a footnote
    /// section.
    notes: Option<Rc<Cell<Notes>>>,
    /// Whether the output ends with a footnote reference, where a `(` would
    /// make it a link and a `:` a definition.
    after_note: bool,
}

impl Inline {
    fn new(place: Place, link: bool) -> Self {
        Self {
            place,
            out: String::new(),
            run: String::new(),
            space: false,
            brk: 0,
            lead: false,
            lead_break: 0,
            link,
            task: false,
            notes: None,
            after_note: false,
        }
    }

    /// An empty writer for one element's content.
    fn child(&self, link: bool) -> Self {
        let mut child = Self::new(self.place, self.link || link);
        child.notes.clone_from(&self.notes);
        child
    }

    fn started(&self) -> bool {
        !self.out.is_empty() || !self.run.is_empty()
    }

    /// Writes the space or the breaks held before the content that comes next.
    fn settle(&mut self) {
        if self.brk > 0 {
            let breaks = std::mem::take(&mut self.brk);
            self.space = false;
            if self.started() {
                self.flush();
                let one = match self.place {
                    Place::Paragraph => "\\\n",
                    Place::Heading | Place::Cell => "<br />",
                };
                for _ in 0..breaks {
                    self.out.push_str(one);
                }
            } else {
                self.lead_break += breaks;
            }
        } else if self.space {
            self.space = false;
            if self.started() {
                self.run.push(' ');
            } else {
                self.lead = true;
            }
        }
        self.task = self.task && !self.started();
    }

    /// Adds text, collapsing its whitespace.
    fn text(&mut self, text: &str) {
        work(text.len());
        for ch in text.chars() {
            if css_space(ch) {
                if self.brk == 0 {
                    self.space = true;
                }
            } else {
                self.settle();
                self.run.push(ch);
            }
        }
    }

    /// Writes `markup` after the text before it.
    fn markup(&mut self, markup: &str) {
        self.settle();
        self.flush();
        self.open(markup);
    }

    /// Writes `open`, the start of an element's Markdown, onto the output. A
    /// `!` right before a link's `[` would make the link an image, so it gets
    /// a backslash.
    fn open(&mut self, open: &str) {
        // A link right after a footnote reference would make the two one
        // reference link, so an empty comment goes between them.
        if std::mem::take(&mut self.after_note) && open.starts_with('[') && !open.starts_with("[^")
        {
            self.out.push_str(CUT);
        }
        if open.starts_with('[') && self.out.ends_with('!') {
            self.out.pop();
            self.out.push_str("\\!");
        }
        // A code span right after another would join its fence to the one
        // before, as `` `a``b` `` is one span, so an empty comment goes
        // between them.
        if open.starts_with('`') && self.out.ends_with('`') {
            self.out.push_str(CUT);
        }
        self.out.push_str(open);
    }

    /// `count` line breaks, held until text follows them.
    fn hard_breaks(&mut self, count: usize) {
        if count > 0 {
            self.brk += count;
            self.space = false;
        }
    }

    /// Escapes the held text onto the output.
    fn flush(&mut self) {
        if self.run.is_empty() {
            return;
        }
        if std::mem::take(&mut self.after_note) && self.run.starts_with(['(', ':']) {
            self.out.push('\\');
        }
        let line_start = self.out.is_empty() || self.out.ends_with('\n');
        escape_into(
            &self.run,
            line_start,
            self.place == Place::Heading,
            !self.link,
            &mut self.out,
        );
        self.run.clear();
    }

    fn finish(mut self) -> Piece {
        self.flush();
        Piece {
            text: self.out,
            lead: self.lead,
            lead_break: self.lead_break,
            trail: self.space,
            trail_break: self.brk,
        }
    }

    /// The whole content of a paragraph, heading or cell, with the breaks at
    /// its two ends written as `<br />`. Its spaces at the ends are dropped,
    /// as a browser drops them. A browser shows no line for one break alone
    /// at the end of a block with content before it, so that break is
    /// dropped. Two or more show a line, and all are written, so the
    /// Markdown's own HTML gives the same Markdown again.
    fn whole(self) -> String {
        let piece = self.finish();
        let mut text = "<br />".repeat(piece.lead_break);
        text.push_str(&piece.text);
        let trail = if text.is_empty() || piece.trail_break != 1 {
            piece.trail_break
        } else {
            0
        };
        text.push_str(&"<br />".repeat(trail));
        text
    }

    /// Writes a child's piece, with `open` and `close` around its text.
    fn wrap(&mut self, piece: &Piece, open: &str, close: &str) {
        if piece.lead_break > 0 {
            self.hard_breaks(piece.lead_break);
        } else if piece.lead && self.brk == 0 {
            self.space = true;
        }
        if !piece.text.is_empty() {
            work(piece.text.len());
            self.settle();
            self.flush();
            self.open(open);
            self.out.push_str(&piece.text);
            self.out.push_str(close);
        }
        if piece.trail_break > 0 {
            self.hard_breaks(piece.trail_break);
        } else if piece.trail && self.brk == 0 {
            self.space = true;
        }
    }

    fn nodes<'a>(&mut self, nodes: impl IntoIterator<Item = &'a Parsed>, depth: usize) {
        for node in nodes {
            self.node(node, depth);
        }
    }

    fn node(&mut self, node: &Parsed, depth: usize) {
        match node.kind {
            Kind::Text => self.text(&node.text),
            Kind::Element if depth > DEPTH => self.text(&text_content(node, " ")),
            Kind::Element => self.element(node, depth + 1),
            _ => {}
        }
    }

    fn element(&mut self, node: &Parsed, depth: usize) {
        match node.name.as_str() {
            "br" => self.hard_breaks(1),
            "wbr" => {}
            "a" => self.link(node, depth),
            "img" => self.image(node),
            "em" | "i" => self.emphasis(node, depth, "*"),
            "strong" | "b" => self.emphasis(node, depth, "**"),
            "del" | "s" => self.emphasis(node, depth, "~~"),
            "code" => self.code(node),
            // An input that is not a task marker writes nothing, so it leaves
            // the space held before it for the text after it.
            "input" => {
                if self.task && !self.started() {
                    self.settle();
                    self.task = false;
                    let mark = if attribute(node, "checked").is_some() {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    self.out.push_str(mark);
                    self.space = true;
                }
            }
            "span" if node.attributes.is_empty() => self.nodes(&node.children, depth),
            "sup" if self.reference(node) => {}
            name if is_block(name) => {
                if self.brk == 0 {
                    self.space = true;
                }
                self.nodes(&node.children, depth);
                if self.brk == 0 {
                    self.space = true;
                }
            }
            _ => {
                let mut inner = self.child(false);
                inner.nodes(&node.children, depth);
                let piece = inner.finish();
                let (open, close) = tags(node);
                self.wrap(&piece, &open, close.as_deref().unwrap_or(""));
            }
        }
    }

    /// `*`, `**` or `~~` around the content when Markdown reads them as
    /// emphasis there, and the element's tags when it would not.
    fn emphasis(&mut self, node: &Parsed, depth: usize, delimiter: &str) {
        let mut inner = self.child(false);
        inner.nodes(&node.children, depth);
        let piece = inner.finish();
        let word = |ch: Option<char>| ch.is_some_and(char::is_alphanumeric);
        // The held space goes out first, so the check below reads what is
        // right before the delimiter.
        if piece.lead_break > 0 {
            self.hard_breaks(piece.lead_break);
        } else if piece.lead && self.brk == 0 {
            self.space = true;
        }
        if !piece.text.is_empty() {
            self.settle();
            self.flush();
        }
        let plain = word(piece.text.chars().next())
            && word(piece.text.chars().next_back())
            && !self.out.ends_with(['*', '~', '_'])
            && !inside_word(&piece.text, delimiter.as_bytes()[0]);
        let bare = Piece {
            text: piece.text,
            lead: false,
            lead_break: 0,
            trail: piece.trail,
            trail_break: piece.trail_break,
        };
        if plain {
            self.wrap(&bare, delimiter, delimiter);
        } else {
            let (open, close) = tags(node);
            self.wrap(&bare, &open, close.as_deref().unwrap_or(""));
        }
    }

    /// Writes `node`, a `sup`, as the footnote reference `[^n]` when it is
    /// the renderer's reference to note `n` of the document's footnote
    /// section, and returns whether it did. A reference inside a link's text
    /// is not one, since a link cannot hold a link.
    fn reference(&mut self, node: &Parsed) -> bool {
        let Some(notes) = self.notes.clone() else {
            return false;
        };
        if self.link {
            return false;
        }
        let [link] = node.children.as_slice() else {
            return false;
        };
        let [text] = link.children.as_slice() else {
            return false;
        };
        if !node.attributes.is_empty()
            || link.kind != Kind::Element
            || link.name != "a"
            || text.kind != Kind::Text
        {
            return false;
        }
        let Ok(number) = text.text.parse::<usize>() else {
            return false;
        };
        let mut state = notes.get();
        let id = format!("md-fnref-{number}");
        let href = format!("#md-fn-{number}");
        let attributes_match =
            link.attributes.iter().all(|(key, value)| {
                (key == "href" && *value == href) || (key == "id" && *value == id)
            }) && attribute(link, "href").is_some();
        if number == 0
            || number > state.count
            || text.text != number.to_string()
            || !attributes_match
        {
            return false;
        }
        if number > state.seen + 1 {
            state.broken = true;
        } else if number == state.seen + 1 {
            state.seen = number;
        }
        notes.set(state);
        self.markup(&format!("[^{number}]"));
        self.after_note = true;
        true
    }

    /// A link, or its text alone when it has no `href`: the sanitizer
    /// removed an `href` it does not allow.
    fn link(&mut self, node: &Parsed, depth: usize) {
        let Some(href) = attribute(node, "href") else {
            self.nodes(&node.children, depth);
            return;
        };
        let mut inner = self.child(true);
        inner.nodes(&node.children, depth);
        let mut piece = inner.finish();
        let mut close = String::from("](");
        destination(href, &mut close);
        if let Some(title) = attribute(node, "title") {
            quoted_title(title, &mut close);
        }
        close.push(')');
        if piece.text.is_empty() {
            // `wrap` writes nothing for an empty text, and an empty link is
            // still a link. Text with breaks alone holds them as its trailing
            // breaks. They go in front of the link, which the browser shows
            // as nothing, so a break alone in its block stays `<br />`.
            let breaks = piece.lead_break + piece.trail_break;
            if breaks > 0 {
                self.hard_breaks(breaks);
            } else if piece.lead && self.brk == 0 {
                self.space = true;
            }
            self.markup("[");
            self.out.push_str(&close);
            piece.lead = false;
            piece.lead_break = 0;
            if breaks > 0 {
                piece.trail = false;
                piece.trail_break = 0;
            }
            self.wrap(&piece, "", "");
            return;
        }
        self.wrap(&piece, "[", &close);
    }

    /// An image, or nothing when it has no `src`.
    fn image(&mut self, node: &Parsed) {
        let Some(src) = attribute(node, "src") else {
            return;
        };
        let mut out = String::from("![");
        let alt: String = attribute(node, "alt")
            .unwrap_or("")
            .chars()
            .map(|ch| if matches!(ch, '\n' | '\r') { ' ' } else { ch })
            .collect();
        escape_into(&alt, false, false, false, &mut out);
        out.push_str("](");
        destination(src, &mut out);
        if let Some(title) = attribute(node, "title") {
            quoted_title(title, &mut out);
        }
        out.push(')');
        self.markup(&out);
    }

    /// A code span. Its whitespace collapses as the browser shows it, and
    /// its fence is one backtick longer than the longest run inside.
    fn code(&mut self, node: &Parsed) {
        let raw = text_content(node, " ");
        let mut code = String::with_capacity(raw.len());
        let mut space = false;
        for ch in raw.chars() {
            if css_space(ch) {
                space = true;
            } else {
                if space {
                    code.push(' ');
                    space = false;
                }
                code.push(ch);
            }
        }
        if space {
            code.push(' ');
        }
        // Markdown has no empty code span, and one of spaces alone is read as
        // it is, without the space at each end taken off.
        if code.is_empty() {
            return;
        }
        if code == " " {
            self.markup("` `");
            return;
        }
        let mut longest = 0;
        let mut run = 0;
        for ch in code.chars() {
            if ch == '`' {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
        let fence = "`".repeat(longest + 1);
        let pad = code.starts_with(['`', ' ']) || code.ends_with(['`', ' ']);
        let mut out = String::with_capacity(code.len() + 2 * fence.len() + 2);
        out.push_str(&fence);
        if pad {
            out.push(' ');
        }
        out.push_str(&code);
        if pad {
            out.push(' ');
        }
        out.push_str(&fence);
        self.markup(&out);
    }
}

/// A link destination, percent-encoded as the renderer writes an `href`
/// ([`super::encoded_href`]), so the Markdown renders to the same `href` and
/// `fromHtml` over that HTML writes the same destination. A browser's URL
/// parser first removes the control characters and spaces at each end and
/// every tab, line feed and carriage return, and so does this: `\x13http:`
/// is `http:`, where `%13http:` would have no scheme and the renderer would
/// drop the link. An `&` that would start an entity gets a backslash. A
/// destination that is empty or has a parenthesis is written in `<` and `>`,
/// where the encoding leaves no `<`, `>` or `\` to escape.
fn destination(url: &str, out: &mut String) {
    let url: String = url
        .trim_matches(|ch: char| ch <= ' ')
        .chars()
        .filter(|ch| !matches!(ch, '\t' | '\n' | '\r'))
        .collect();
    let encoded = super::encoded_href(&url);
    let angled = encoded.is_empty() || encoded.contains(['(', ')']);
    if angled {
        out.push('<');
    }
    for (at, ch) in encoded.char_indices() {
        if ch == '&' && entity_follows(&encoded[at + 1..]) {
            out.push('\\');
        }
        out.push(ch);
    }
    if angled {
        out.push('>');
    }
}

/// Whether `rest`, the text after an `&`, starts like an entity: letters,
/// digits or `#`, then `;`.
fn entity_follows(rest: &str) -> bool {
    let name = rest
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '#'))
        .unwrap_or(rest.len());
    name > 0 && rest[name..].starts_with(';')
}

/// A link title in double quotes, after a space, or nothing for an empty
/// title. The HTML render writes no `title` for an empty one, so writing `""`
/// would not survive a second pass.
fn quoted_title(title: &str, out: &mut String) {
    if title.is_empty() {
        return;
    }
    out.push_str(" \"");
    for ch in title.chars() {
        match ch {
            '"' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            '\n' | '\r' => out.push(' '),
            _ => out.push(ch),
        }
    }
    out.push('"');
}

/// Whether `text` has a run of `mark` with a word character on each side,
/// as in `a*b`. Such a run can both open and close emphasis, so it could pair
/// with the delimiter written around `text`: `*a*b*c*` reads as two
/// emphases, not one around another. A character that is not whitespace and
/// not ASCII punctuation counts as a word character, which is conservative
/// for Unicode punctuation and symbols: they only cost the element its tags.
fn inside_word(text: &str, mark: u8) -> bool {
    let joins =
        |ch: Option<char>| ch.is_some_and(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation());
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(found) = bytes[at..].iter().position(|&byte| byte == mark) {
        let start = at + found;
        let end = start
            + bytes[start..]
                .iter()
                .take_while(|&&byte| byte == mark)
                .count();
        if joins(text[..start].chars().next_back()) && joins(text[end..].chars().next()) {
            return true;
        }
        at = end;
    }
    false
}

/// Whether `line` is a thematic break: three or more of one of `-`, `*` and
/// `_`, with only spaces and tabs between them.
fn thematic_break(line: &str) -> bool {
    let mut marks = line.chars().filter(|ch| !matches!(ch, ' ' | '\t'));
    let Some(mark) = marks.next() else {
        return false;
    };
    matches!(mark, '-' | '*' | '_')
        && marks
            .try_fold(1, |count, ch| (ch == mark).then_some(count + 1))
            .is_some_and(|count| count >= 3)
}

/// Whether `ch` may stand right before the `@` of an email autolink.
fn local_part(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '+' | '-' | '_')
}

/// `text` escaped onto `out`, so that no character of it is read as Markdown.
///
/// `\`, `` ` ``, `*`, `_`, `[`, `]`, `<`, `>`, `&`, `|` and `~` always get a
/// backslash. At the start of a line `#`, `-`, `+`, `=` and `:` do too, and
/// so does the `.` or `)` after a number. A `:` there would start a table's
/// delimiter row, such as `:-`. In a heading every `#` does. Where
/// `cut` is set, a text that would be an autolink is cut by [`CUT`].
fn escape_into(text: &str, line_start: bool, heading: bool, cut: bool, out: &mut String) {
    work(text.len());
    let bytes = text.as_bytes();
    // The byte after a number at the start of a line, when it is `.` or `)`.
    let numbered = if line_start {
        let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
        (digits > 0 && matches!(bytes.get(digits), Some(b'.' | b')'))).then_some(digits)
    } else {
        None
    };
    let mut previous: Option<char> = None;
    for (at, ch) in text.char_indices() {
        if cut {
            let www = ch == '.' && bytes[..at].ends_with(b"www");
            let scheme = ch == ':'
                && text[at..].starts_with("://")
                && previous.is_some_and(|before| before.is_ascii_alphanumeric());
            let email = ch == '@'
                && previous.is_some_and(local_part)
                && text[at + 1..].chars().next().is_some_and(|after| {
                    after.is_ascii_alphanumeric() || matches!(after, '-' | '_' | '.')
                });
            if www || scheme || email {
                out.push_str(CUT);
            }
        }
        let escaped = matches!(
            ch,
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '&' | '|' | '~'
        ) || (heading && ch == '#')
            || (at == 0 && line_start && matches!(ch, '#' | '-' | '+' | '=' | ':'))
            || numbered == Some(at);
        if escaped {
            out.push('\\');
        }
        out.push(ch);
        previous = Some(ch);
    }
}
