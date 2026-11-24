//! The editor finds, renames and quick-fixes every `@see` target and
//! `@throws` name, and a moved file or folder rewrites the doc comment tags
//! that name it.
//!
//! Each answer here is a site in a file on disk, so it names the machine the
//! test ran on. A `.lspt` case cannot freeze that, so the claims are written
//! here.

use std::fs;
use std::path::PathBuf;

use nvs_diagnostics::PositionEncoding;
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::query::{self, UseKind};
use nvs_lsp::rename::{self, Rewrite};
use nvs_lsp::{Documents, Site, Workspace, actions, analyse, analyse_file, uri_of, workspace};

/// The Markdown file the shop's `@see` file target points into.
const GUIDE: &str = "# Guide\n\n## Paying by card\n\nCards are charged once.\n\n\
                     ## Refunds\n\nRefunds take a week.\n";

/// The document every test reads. `total` is private and used nowhere, so
/// only its declaration and its `@see` name it.
const SHOP: &str = "<?nvs\n\
                    final class Declined extends RuntimeError {\n\
                    \x20   public string $reason = '';\n\
                    }\n\
                    \n\
                    /// A shop that sells things.\n\
                    ///\n\
                    /// @see Shop::total\n\
                    /// @see Declined::$reason\n\
                    /// @see docs/guide.md#refunds\n\
                    /// @example examples/run.nvs\n\
                    final class Shop\n\
                    {\n\
                    \x20   /// Charges the card.\n\
                    \x20   ///\n\
                    \x20   /// @throws Declined when the card is declined\n\
                    \x20   public function charge(): int { return $this->total(); }\n\
                    \n\
                    \x20   private function total(): int { return 1; }\n\
                    }\n\
                    echo (new Shop())->charge();\n";

/// A project folder with the shop, its guide, its example and a file no tag
/// names, and the workspace built over it.
struct Project {
    dir: nvs_repo::Scratch,
    workspace: Workspace,
}

impl Project {
    fn new(name: &str) -> Self {
        let dir = nvs_repo::scratch(&format!("lsp-see-edits-{name}"));
        for (path, text) in [
            ("shop.nvs", SHOP),
            ("docs/guide.md", GUIDE),
            ("examples/run.nvs", "<?nvs\necho 1;\n"),
            ("notes.txt", "Nothing names this file.\n"),
        ] {
            let at = dir.join(path);
            fs::create_dir_all(at.parent().expect("a file has a directory"))
                .expect("a writable scratch directory");
            fs::write(at, text).expect("a writable scratch file");
        }
        let workspace = workspace(&dir);
        Self { dir, workspace }
    }

    fn at(&self, path: &str) -> PathBuf {
        self.dir.join(path)
    }

    /// Every rewrite a move of each `(old, new)` path makes, as the text it
    /// replaces and the text it writes.
    fn moved(&self, moves: &[(&str, &str)]) -> Vec<(String, String)> {
        let moves: Vec<(PathBuf, PathBuf)> = moves
            .iter()
            .map(|(old, new)| (self.at(old), self.at(new)))
            .collect();
        written(&rename::moved(
            self.workspace.documents(),
            self.workspace.index(),
            &moves,
        ))
    }

    /// Every rewrite a rename of what the cursor at `needle` plus `into` is
    /// on makes, as the text it replaces and the text it writes.
    fn renamed(&self, needle: &str, into: usize, new: &str) -> Vec<(String, String)> {
        let documents = self.workspace.documents();
        let analysed = analyse_file(documents, &self.at("shop.nvs"), 0).expect("an analysis");
        let renamed = rename::rename(
            documents,
            self.workspace.index(),
            &analysed,
            offset_of(needle, into),
            new,
        )
        .expect("a rename");
        written(&renamed.rewrites)
    }
}

/// The text a site covers.
fn text_of(site: &Site) -> String {
    let text = fs::read_to_string(&site.path).expect("an indexed file");
    text[site.start as usize..site.end as usize].to_owned()
}

/// Each rewrite as the text it replaces and the text it writes.
fn written(rewrites: &[Rewrite]) -> Vec<(String, String)> {
    rewrites
        .iter()
        .map(|rewrite| (text_of(&rewrite.site), rewrite.text.clone()))
        .collect()
}

/// The byte offset of `needle` in [`SHOP`], plus `into`.
fn offset_of(needle: &str, into: usize) -> u32 {
    let at = SHOP.find(needle).expect("the needle is in the document");
    u32::try_from(at + into).expect("a test document is short")
}

/// Whether `site` starts inside the doc comment line `line` of [`SHOP`].
fn on_line(site: &Site, line: &str) -> bool {
    let start = SHOP.find(line).expect("the line is in the document");
    (start..start + line.len()).contains(&(site.start as usize))
}

#[test]
fn references_list_a_see_target_without_counting_it() {
    let project = Project::new("references");
    let index = project.workspace.index();
    let found = query::references(index, "Shop::total", false);
    let docs: Vec<&Site> = found
        .iter()
        .filter(|reference| reference.kind == UseKind::Doc)
        .map(|reference| reference.site)
        .collect();
    assert_eq!(docs.len(), 1, "{found:?}");
    assert!(on_line(docs[0], "/// @see Shop::total"), "{docs:?}");
    assert_eq!(text_of(docs[0]), "total");
    // The CodeLens count and unused-member dimming read the occurrences, and
    // a doc comment is not one of them: the call in `charge` is the only use.
    let uses = index.occurrences("Shop::total");
    assert_eq!(uses.len(), 1, "{uses:?}");
    assert!(!on_line(&uses[0].site, "/// @see Shop::total"));
}

#[test]
fn renaming_a_member_rewrites_its_see_targets() {
    let project = Project::new("member");
    let edits = project.renamed("function total", 9, "sum");
    assert!(
        edits
            .iter()
            .all(|edit| edit == &("total".to_owned(), "sum".to_owned())),
        "{edits:?}"
    );
    // The declaration, the call and the `@see`.
    assert_eq!(edits.len(), 3, "{edits:?}");

    let property = project.renamed("$reason = ", 1, "cause");
    assert_eq!(
        property,
        [
            ("$reason".to_owned(), "$cause".to_owned()),
            ("$reason".to_owned(), "$cause".to_owned()),
        ]
    );
}

#[test]
fn renaming_a_class_rewrites_its_see_and_throws_targets() {
    let project = Project::new("class");
    let edits = project.renamed("class Declined", 6, "Refused");
    assert!(
        edits
            .iter()
            .all(|edit| edit == &("Declined".to_owned(), "Refused".to_owned())),
        "{edits:?}"
    );
    // The declaration, the class half of `@see Declined::$reason` and the
    // `@throws` name.
    assert_eq!(edits.len(), 3, "{edits:?}");
}

#[test]
fn prepare_rename_on_a_see_target_renames_the_member() {
    let project = Project::new("prepare");
    let documents = project.workspace.documents();
    let index = project.workspace.index();
    let analysed = analyse_file(documents, &project.at("shop.nvs"), 0).expect("an analysis");
    let at = offset_of("@see Shop::total", 12);
    let site = rename::prepare(index, &analysed, at).expect("the target can be renamed");
    assert_eq!(text_of(&site), "total");
    let class = rename::prepare(index, &analysed, offset_of("@see Shop::total", 6))
        .expect("the class half can be renamed");
    assert_eq!(text_of(&class), "Shop");

    assert_eq!(
        project.renamed("@see Shop::total", 12, "sum"),
        project.renamed("function total", 9, "sum")
    );
}

#[test]
fn renaming_a_file_rewrites_the_see_and_example_tags_that_name_it() {
    let project = Project::new("file");
    assert_eq!(
        project.moved(&[("docs/guide.md", "docs/handbook.md")]),
        [("docs/guide.md".to_owned(), "docs/handbook.md".to_owned())]
    );
    assert_eq!(
        project.moved(&[("examples/run.nvs", "examples/start.nvs")]),
        [(
            "examples/run.nvs".to_owned(),
            "examples/start.nvs".to_owned()
        )]
    );
    // The file that writes the tags moves too, so its paths start one folder
    // further down.
    assert_eq!(
        project.moved(&[("shop.nvs", "src/shop.nvs")]),
        [
            ("docs/guide.md".to_owned(), "../docs/guide.md".to_owned()),
            (
                "examples/run.nvs".to_owned(),
                "../examples/run.nvs".to_owned()
            ),
        ]
    );
}

#[test]
fn renaming_a_folder_rewrites_the_see_targets_inside_it() {
    let project = Project::new("folder");
    assert_eq!(
        project.moved(&[("docs", "manual")]),
        [("docs/guide.md".to_owned(), "manual/guide.md".to_owned())]
    );
}

#[test]
fn a_file_rename_no_tag_names_answers_nothing() {
    let project = Project::new("nothing");
    assert_eq!(project.moved(&[("notes.txt", "todo.txt")]), []);
}

#[test]
fn a_missing_heading_offers_the_closest_one_as_a_quick_fix() {
    let project = Project::new("quick-fix");
    let source = SHOP.replace("#refunds", "#refnds");
    let path = project.at("shop.nvs");
    fs::write(&path, &source).expect("a writable scratch file");
    let uri = uri_of(&path).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.clone());
    let analysed = analyse(&documents, &uri).expect("an open document analyses");
    let at = u32::try_from(source.find("#refnds").expect("the anchor") + 2).expect("short");
    let offered: Vec<String> = actions::at(
        &analysed,
        &documents,
        None,
        &CompletionFiles::default(),
        at,
        at,
        actions::Kind::QuickFix,
        PositionEncoding::Utf8,
    )
    .into_iter()
    .map(|action| action.replacement)
    .collect();
    assert!(
        offered.iter().any(|text| text.ends_with("refunds")),
        "{offered:?}"
    );
}
