//! `rule:testing/tags-and-shards-select-what-runs`' two selectors that read
//! the source: `--since <ref>`, which runs the tests a change can reach, and
//! `<file>:<line>`, which runs the test written at that line. Both become a
//! [`Pick`] on the run's [`super::Selection`].
//!
//! # The reached map
//!
//! A run that measures per-test coverage writes, per entry path, a text file
//! in the data folder beside the failed-tests file ([`map_file`]):
//!
//! ```text
//! nvs reached 1
//! test Shop\CartTest::totals
//!   Shop\Cart::total
//! function <hash> Shop\Cart::total
//! class <hash> Shop\CartTest
//! file <hash> <canonical path>
//! ```
//!
//! Each hash is blake3. A `function` line hashes a method body's text. A
//! `class` line hashes a test class's text outside its method bodies, and a
//! `file` line a program file's text outside its test classes and method
//! bodies. Those two read every run of whitespace as one space, so a blank
//! line between two classes changes neither.
//!
//! A function is a method body, named by its `Class::method` label. A
//! statement in an anonymous function or an anonymous class counts for the
//! method it is written in. A statement outside every method body, such as a
//! property hook or a property's default, counts for no function: a change
//! there is outside a body, so it runs every test anyway. A run that left a
//! test out keeps that test's entry from the stored map, so a `--since` run
//! never forgets the tests it did not run. An entry for a test the program no
//! longer has is dropped.
//!
//! # What `--since` selects
//!
//! A function is changed when a changed line is inside its body, after the
//! line its `{` is on, or when its body's hash differs from the map's. A test
//! runs when it reached a changed function, when the map does not name it,
//! or when a line of its own method changed. A changed line in a test class
//! outside its test methods, such as a hook, a fixture or a property, runs
//! every test of that class, and so does a class hash that differs. Every
//! selected test runs, and [`select`] says why, when there is no map, when a
//! changed line that is not blank is outside every method body and test
//! class, when a program file is not in the map or its hash differs, or when
//! a file in the program's folder that is not a `.nvs` file changed or was
//! deleted. The first line of a method is outside its body, because a
//! signature or an attribute changes which code a call reaches. A line that
//! was only deleted sits between two lines, and is inside a body only when
//! both are.
//!
//! The hashes keep the map honest when it was written from a working tree
//! that has changed since: a body edited and then put back the way the ref
//! has it is not in git's diff, and its hash still differs.
//!
//! # What it spends
//!
//! Per run that measures coverage: one label per method body and one line
//! range per test class while the run lasts, and one label set per test when
//! the map is written. On disk, one line per test and per function it
//! reached, and one line per program file, per entry path. It grows with the
//! program and never with runs, because every run replaces it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nvs_diagnostics::{SourceFile, SourceId, Span};
use nvs_syntax::ast::{ClassDecl, ClassMemberKind, Stmt, StmtKind};

use super::since::{Changed, Lines};

/// The first line of every map file. A file without it is no map.
const HEADER: &str = "nvs reached 1";

/// The tests a `--since` or `<file>:<line>` selection picks. A test runs when
/// any of the sets names it.
#[derive(Debug, Default)]
pub(crate) struct Pick {
    /// Test classes every test of which runs.
    classes: BTreeSet<String>,
    /// `Class::method`, every data row of it included.
    methods: BTreeSet<String>,
    /// `Class::label`, as the reached map names a test.
    tests: BTreeSet<String>,
    /// Under `--since`, every test the map names. A test it does not name
    /// runs.
    known: Option<BTreeSet<String>>,
}

impl Pick {
    /// Whether the test `label` of `method`, in `class`, runs.
    pub(super) fn picks(&self, class: &str, method: &str, label: &str) -> bool {
        let test = format!("{class}::{label}");
        self.classes.contains(class)
            || self.methods.contains(&format!("{class}::{method}"))
            || self.tests.contains(&test)
            || self
                .known
                .as_ref()
                .is_some_and(|known| !known.contains(&test))
    }
}

/// One method body.
struct Body {
    label: String,
    span: Span,
    /// The line of its `{` and of its `}`, from 1.
    open: usize,
    close: usize,
}

/// One test class as it is written.
struct TestClass {
    label: String,
    span: Span,
    file: SourceId,
    first: usize,
    last: usize,
    /// Each test method's name and its lines, attributes included.
    tests: Vec<(String, usize, usize)>,
}

/// Where the program's method bodies and test classes are written.
pub(super) struct Layout {
    bodies: Vec<Body>,
    classes: Vec<TestClass>,
    /// Each program file read from disk, by its canonical path.
    files: Vec<(SourceId, PathBuf)>,
}

/// The line of `pos` in `file`, from 1.
fn line_at(file: &SourceFile, pos: u32) -> usize {
    file.line_index(pos) + 1
}

/// The first and last line of `span`.
fn lines_of(file: &SourceFile, span: Span) -> (usize, usize) {
    (
        line_at(file, span.start),
        line_at(file, span.end.saturating_sub(1).max(span.start)),
    )
}

impl Layout {
    /// Walks every program file's classes, at the top and in a braced
    /// `namespace`.
    pub(super) fn of(checked: &crate::Checked) -> Self {
        // Each test class's label and the name span of each of its tests.
        let mut test_names: Vec<(&str, Vec<(&str, Span)>)> = Vec::new();
        for class in checked.exprs.test_classes() {
            let mut names: Vec<(&str, Span)> = checked
                .exprs
                .tests(class)
                .unwrap_or_default()
                .iter()
                .map(|case| (case.method.as_str(), case.span))
                .collect();
            names.extend(
                checked
                    .exprs
                    .properties(class)
                    .unwrap_or_default()
                    .iter()
                    .map(|case| (case.method.as_str(), case.span)),
            );
            test_names.push((class, names));
        }
        let mut layout = Self {
            bodies: Vec::new(),
            classes: Vec::new(),
            files: Vec::new(),
        };
        for file in checked.program_files() {
            if let Some(path) = file.src.path().and_then(|p| std::fs::canonicalize(p).ok()) {
                layout.files.push((file.src.id(), path));
            }
            layout.walk(checked, file.src, file.stmts, &test_names);
        }
        layout
    }

    fn walk(
        &mut self,
        checked: &crate::Checked,
        src: &SourceFile,
        stmts: &[Stmt],
        test_names: &[(&str, Vec<(&str, Span)>)],
    ) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::ClassDecl(class) => self.class(checked, src, class, test_names),
                StmtKind::NamespaceDecl(namespace) => {
                    if let Some(body) = &namespace.body {
                        self.walk(checked, src, &body.stmts, test_names);
                    }
                }
                _ => {}
            }
        }
    }

    fn class(
        &mut self,
        checked: &crate::Checked,
        src: &SourceFile,
        class: &ClassDecl,
        test_names: &[(&str, Vec<(&str, Span)>)],
    ) {
        let inside = |span: Span| {
            span.file == class.span.file
                && class.span.start <= span.start
                && span.end <= class.span.end
        };
        let tested = test_names
            .iter()
            .find(|(_, names)| names.iter().any(|&(_, span)| inside(span)));
        let mut tests = Vec::new();
        for member in &class.members {
            let ClassMemberKind::Method(method) = &member.kind else {
                continue;
            };
            if let Some((_, names)) = tested
                && let Some(&(name, _)) = names.iter().find(|&&(_, span)| span == method.name)
            {
                let (first, last) = lines_of(src, member.span);
                tests.push((name.to_owned(), first, last));
            }
            if let (Some(body), Some(label)) =
                (&method.body, checked.exprs.method_label(method.name))
            {
                let (open, close) = lines_of(src, body.span);
                self.bodies.push(Body {
                    label: label.to_owned(),
                    span: body.span,
                    open,
                    close,
                });
            }
        }
        if let Some((label, _)) = tested {
            let (first, last) = lines_of(src, class.span);
            self.classes.push(TestClass {
                label: (*label).to_owned(),
                span: class.span,
                file: src.id(),
                first,
                last,
                tests,
            });
        }
    }

    /// The label of the method body `stmt` is written in.
    fn label_of(&self, stmt: Span) -> Option<&str> {
        self.bodies
            .iter()
            .find(|body| {
                body.span.file == stmt.file
                    && body.span.start <= stmt.start
                    && stmt.end <= body.span.end
            })
            .map(|body| body.label.as_str())
    }

    fn file_of(&self, path: &Path) -> Option<SourceId> {
        self.files
            .iter()
            .find(|(_, file)| file == path)
            .map(|&(id, _)| id)
    }

    /// The hashes of the program as it is now: see [`Hashes`].
    fn hashes(&self, checked: &crate::Checked) -> Hashes {
        let mut hashes = Hashes::default();
        let text = |span: Span| {
            checked
                .map
                .get(span.file)
                .and_then(|file| file.text().get(span.start as usize..span.end as usize))
                .unwrap_or_default()
        };
        for body in &self.bodies {
            hashes
                .functions
                .insert(body.label.clone(), hash_of(text(body.span)));
        }
        for class in &self.classes {
            let cut: Vec<Span> = self
                .bodies
                .iter()
                .map(|body| body.span)
                .filter(|span| within(*span, class.span))
                .collect();
            hashes.classes.insert(
                class.label.clone(),
                outside(text(class.span), class.span.start, &cut),
            );
        }
        for (id, path) in &self.files {
            let Some(source) = checked.map.get(*id) else {
                continue;
            };
            let cut: Vec<Span> = self
                .classes
                .iter()
                .map(|class| class.span)
                .chain(self.bodies.iter().map(|body| body.span))
                .filter(|span| span.file == *id)
                .collect();
            hashes
                .files
                .insert(path.clone(), outside(source.text(), 0, &cut));
        }
        hashes
    }
}

/// Whether `inner` is inside `outer`.
fn within(inner: Span, outer: Span) -> bool {
    inner.file == outer.file && outer.start <= inner.start && inner.end <= outer.end
}

/// The hash of `text`, which starts at byte `base` of its file, with every
/// span of `cut` taken out and every run of whitespace read as one space.
fn outside(text: &str, base: u32, cut: &[Span]) -> String {
    let mut cut: Vec<(usize, usize)> = cut
        .iter()
        .map(|span| {
            (
                span.start.saturating_sub(base) as usize,
                span.end.saturating_sub(base) as usize,
            )
        })
        .collect();
    cut.sort_unstable();
    let mut kept = String::with_capacity(text.len());
    let mut at = 0;
    for (start, end) in cut {
        if start > at {
            kept.push_str(text.get(at..start).unwrap_or_default());
            kept.push(' ');
        }
        at = at.max(end);
    }
    kept.push_str(text.get(at..).unwrap_or_default());
    let words: Vec<&str> = kept.split_whitespace().collect();
    hash_of(&words.join(" "))
}

fn hash_of(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex().to_string()
}

/// What the map hashes, so a run can tell what changed since the map was
/// written, whatever git says.
#[derive(Default)]
struct Hashes {
    /// Each program file's canonical path, with the hash of its text outside
    /// its test classes and method bodies.
    files: BTreeMap<PathBuf, String>,
    /// Each test class's label, with the hash of its text outside its method
    /// bodies.
    classes: BTreeMap<String, String>,
    /// Each method body's label, with the hash of its text.
    functions: BTreeMap<String, String>,
}

/// The reached map of one entry path.
#[derive(Default)]
pub(super) struct Map {
    /// Each test's `Class::label`, with the label of every function it reached.
    tests: BTreeMap<String, BTreeSet<String>>,
    hashes: Hashes,
}

/// The reached map's file in the data folder for the program at `entry`, or
/// `None` when there is no data folder. It is named as
/// [`super::failed_tests_file`] names its file.
pub(crate) fn map_file(entry: &Path) -> Option<PathBuf> {
    let folder = nvs_config::data::current()?;
    let canonical = std::fs::canonicalize(entry).unwrap_or_else(|_| entry.to_path_buf());
    let hash = blake3::hash(canonical.to_string_lossy().as_bytes()).to_hex();
    Some(
        folder
            .reached_tests()
            .join(format!("{}.txt", hash.get(..32).unwrap_or(&hash))),
    )
}

/// The map `file` holds, or `None` when no run wrote one.
pub(super) fn read(file: &Path) -> Option<Map> {
    let text = std::fs::read_to_string(file).ok()?;
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return None;
    }
    let mut map = Map::default();
    let mut test: Option<String> = None;
    for line in lines {
        if let Some(name) = line.strip_prefix("test ") {
            map.tests.insert(name.to_owned(), BTreeSet::new());
            test = Some(name.to_owned());
        } else if let Some((kind, rest)) = line.split_once(' ')
            && matches!(kind, "function" | "class" | "file")
        {
            let (hash, name) = rest.split_once(' ')?;
            let hash = hash.to_owned();
            match kind {
                "function" => map.hashes.functions.insert(name.to_owned(), hash),
                "class" => map.hashes.classes.insert(name.to_owned(), hash),
                _ => map.hashes.files.insert(PathBuf::from(name), hash),
            };
            test = None;
        } else if let (Some(label), Some(test)) = (line.strip_prefix("  "), &test) {
            map.tests.get_mut(test)?.insert(label.to_owned());
        }
    }
    Some(map)
}

/// Writes the map of this run to `file`: each test of `cases` with the
/// functions it reached, each test the program still has and this run left
/// out with its entry from the stored map, and every program file's hash. A
/// write that fails is a warning, never a verdict.
pub(super) fn record(
    file: &Path,
    layout: &Layout,
    stmts: &[Span],
    cases: &[super::Case],
    checked: &crate::Checked,
) {
    let mut map = Map::default();
    for case in cases {
        let Some(reached) = &case.reached else {
            continue;
        };
        let labels = reached
            .iter()
            .filter_map(|&stmt| stmts.get(stmt))
            .filter_map(|&span| layout.label_of(span))
            .map(ToOwned::to_owned)
            .collect();
        map.tests
            .insert(format!("{}::{}", case.class, case.method), labels);
    }
    if let Some(stored) = read(file) {
        let current: BTreeSet<String> = super::listing(checked, &super::Selection::default())
            .into_iter()
            .map(|test| format!("{}::{}", test.class, test.method))
            .collect();
        for (test, labels) in stored.tests {
            if current.contains(&test) && !map.tests.contains_key(&test) {
                map.tests.insert(test, labels);
            }
        }
    }
    map.hashes = layout.hashes(checked);
    let mut text = format!("{HEADER}\n");
    for (test, labels) in &map.tests {
        text.push_str(&format!("test {test}\n"));
        for label in labels {
            text.push_str(&format!("  {label}\n"));
        }
    }
    for (label, hash) in &map.hashes.functions {
        text.push_str(&format!("function {hash} {label}\n"));
    }
    for (label, hash) in &map.hashes.classes {
        text.push_str(&format!("class {hash} {label}\n"));
    }
    for (path, hash) in &map.hashes.files {
        text.push_str(&format!("file {hash} {}\n", path.display()));
    }
    let written = file
        .parent()
        .map_or(Ok(()), nvs_config::data::create_private_root)
        .and_then(|()| std::fs::write(file, text));
    if let Err(error) = written {
        eprintln!(
            "warning: could not record which code each test reached in {}: {error}",
            file.display()
        );
    }
}

/// The tests `changed` can reach, by `map`. `folder` is the program's folder,
/// canonical. `Err` is why every selected test runs, as a sentence's end.
pub(super) fn select(
    layout: &Layout,
    checked: &crate::Checked,
    map: Option<Map>,
    changed: &Changed,
    folder: &Path,
) -> Result<Pick, String> {
    let map = map.ok_or("no earlier run recorded which code each test reached")?;
    let mut pick = Pick::default();
    let mut functions: BTreeSet<&str> = BTreeSet::new();
    for (path, lines) in changed.files() {
        let Some(id) = layout.file_of(path) else {
            let ours = path.starts_with(folder);
            let nvs = path.extension().is_some_and(|ext| ext == "nvs");
            if ours && !nvs {
                return Err(format!(
                    "`{}` changed, and it is not a `.nvs` file",
                    shown(path, folder)
                ));
            }
            continue;
        };
        let Some(source) = checked.map.get(id) else {
            continue;
        };
        let name = shown(path, folder);
        let (added, deleted) = match lines {
            Lines::Whole => (vec![(1, source.line_count())], Vec::new()),
            Lines::Ranges { added, deleted } => (added.clone(), deleted.clone()),
            Lines::Gone => continue,
        };
        // A deletion point `p` sits between line `p` and line `p + 1`.
        let text: Vec<&str> = source.text().lines().collect();
        // Each added line is judged alone: git may align one hunk across the end of one method
        // and the start of the next.
        let edits = added
            .into_iter()
            .flat_map(|(first, last)| (first..=last).map(|line| (line, line, false)))
            .chain(deleted.into_iter().map(|p| (p, p + 1, true)));
        for (first, last, point) in edits {
            let mut placed = false;
            if let Some(class) = layout
                .classes
                .iter()
                .find(|class| class.file == id && class.first <= first && last <= class.last)
            {
                placed = true;
                match class
                    .tests
                    .iter()
                    .find(|&&(_, from, to)| from <= first && last <= to)
                {
                    Some((method, ..)) => {
                        pick.methods.insert(format!("{}::{method}", class.label));
                    }
                    None => {
                        pick.classes.insert(class.label.clone());
                    }
                }
            }
            let body = layout.bodies.iter().find(|body| {
                body.span.file == id
                    && match point {
                        true => body.open <= first && last <= body.close,
                        false => body.open < first && last <= body.close,
                    }
            });
            let blank = !point
                && (first..=last).all(|line| {
                    text.get(line.saturating_sub(1))
                        .is_none_or(|text| text.trim().is_empty())
                });
            match body {
                Some(body) => {
                    functions.insert(&body.label);
                }
                None if placed || blank => {}
                None => {
                    return Err(format!(
                        "line {first} of `{name}` changed, and it is outside a method body"
                    ));
                }
            }
        }
    }
    // What changed since the map was written, which git does not see when the map was written
    // from a working tree that has changed since.
    let now = layout.hashes(checked);
    for (path, hash) in &now.files {
        match map.hashes.files.get(path) {
            None => {
                return Err(format!(
                    "`{}` is not in the record of the last run",
                    shown(path, folder)
                ));
            }
            Some(stored) if stored != hash => {
                return Err(format!(
                    "`{}` changed outside a method body since the last run",
                    shown(path, folder)
                ));
            }
            Some(_) => {}
        }
    }
    for (label, hash) in &now.classes {
        if map.hashes.classes.get(label) != Some(hash) {
            pick.classes.insert(label.clone());
        }
    }
    for (label, hash) in &now.functions {
        if map.hashes.functions.get(label) != Some(hash) {
            functions.insert(label);
        }
    }
    for (test, reached) in &map.tests {
        if reached
            .iter()
            .any(|label| functions.contains(label.as_str()))
        {
            pick.tests.insert(test.clone());
        }
    }
    pick.known = Some(map.tests.into_keys().collect());
    Ok(pick)
}

/// The test or test class written at `line` of `file`. `Err` is the error
/// the run stops with.
pub(super) fn at_line(
    layout: &Layout,
    checked: &crate::Checked,
    file: &Path,
    line: usize,
) -> Result<Pick, String> {
    let shown_file = file.display();
    let id = std::fs::canonicalize(file)
        .ok()
        .and_then(|path| layout.file_of(&path))
        .ok_or_else(|| format!("`{shown_file}` is not a file of this program"))?;
    let classes: Vec<&TestClass> = layout.classes.iter().filter(|c| c.file == id).collect();
    let mut pick = Pick::default();
    if let Some(class) = classes
        .iter()
        .find(|class| class.first <= line && line <= class.last)
    {
        match class
            .tests
            .iter()
            .find(|&&(_, from, to)| from <= line && line <= to)
        {
            Some((method, ..)) => pick.methods.insert(format!("{}::{method}", class.label)),
            None => pick.classes.insert(class.label.clone()),
        };
        return Ok(pick);
    }
    let lines = checked.map.get(id).map_or(0, SourceFile::line_count);
    if line == 0 || line > lines {
        return Err(format!(
            "`{shown_file}` has {lines} lines, and no line {line}"
        ));
    }
    let nearest = classes
        .iter()
        .min_by_key(|class| class.first.abs_diff(line).min(class.last.abs_diff(line)));
    Err(match nearest {
        Some(class) => format!(
            "line {line} of `{shown_file}` is not in a test class. The nearest test class is `{}`, on line {}",
            class.label, class.first
        ),
        None => {
            format!("line {line} of `{shown_file}` is not in a test class, and the file has none")
        }
    })
}

/// `path` relative to `folder` when it is inside it, with `/` between parts.
fn shown(path: &Path, folder: &Path) -> String {
    path.strip_prefix(folder)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The file and line of a path argument written as `<file>:<line>`: the path
/// does not exist, it ends in `:` and digits, and the part before the colon
/// exists. A Windows drive letter stays part of the file.
pub(crate) fn split_line(path: &Path) -> Option<(PathBuf, usize)> {
    if path.exists() {
        return None;
    }
    let text = path.to_str()?;
    let (file, line) = text.rsplit_once(':')?;
    if line.is_empty() || !line.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let file = PathBuf::from(file);
    file.is_file().then_some(())?;
    Some((file, line.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pick_runs_a_named_class_method_or_test_and_every_test_the_map_does_not_know() {
        let pick = Pick {
            classes: BTreeSet::from(["A".to_owned()]),
            methods: BTreeSet::from(["B::one".to_owned()]),
            tests: BTreeSet::from(["C::two".to_owned()]),
            known: Some(BTreeSet::from(["C::two".to_owned(), "C::three".to_owned()])),
        };
        assert!(pick.picks("A", "any", "any"));
        assert!(pick.picks("B", "one", "one#2"));
        assert!(pick.picks("C", "two", "two"));
        assert!(!pick.picks("C", "three", "three"));
        assert!(
            pick.picks("C", "four", "four"),
            "a test the map does not know runs"
        );
    }
}
