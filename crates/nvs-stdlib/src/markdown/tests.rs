//! `rule:core-classes/markdown-to-html`'s and `markdown-cost`'s properties
//! over the renderer, with the spec examples vendored under
//! `tests/data/markdown/spec/`.

use super::*;
use crate::html::parse;

/// One spec example: its number, counted from 1 over every example in its
/// file, the extensions its header names, its Markdown and its HTML.
struct Example {
    number: usize,
    header: String,
    markdown: String,
    html: String,
}

/// Every example of the spec file `name`, with `→` read as a tab.
fn examples(name: &str) -> Vec<Example> {
    let path = format!(
        "{}/../../tests/data/markdown/spec/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let fence = "`".repeat(32);
    let mut out = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let Some(header) = line.strip_prefix(&fence) else {
            continue;
        };
        let Some(header) = header.trim().strip_prefix("example") else {
            continue;
        };
        let mut markdown = String::new();
        let mut html = String::new();
        let mut in_html = false;
        for line in lines.by_ref() {
            if line == fence {
                break;
            }
            if line == "." && !in_html {
                in_html = true;
                continue;
            }
            let into = if in_html { &mut html } else { &mut markdown };
            into.push_str(&line.replace('→', "\t"));
            into.push('\n');
        }
        out.push(Example {
            number: out.len() + 1,
            header: header.trim().to_owned(),
            markdown,
            html,
        });
    }
    out
}

/// `html` as a browser builds it, rebuilt through the sanitizer's list and
/// written again.
///
/// Both sides of a spec comparison go through this. It reads `&quot;` and
/// `"` in text as the same character, `<br />` and `<br>` as the same tag,
/// and drops what the sanitizer drops from the spec's own raw HTML.
fn normal(html: &str) -> String {
    source(&rebuilt(&parse(html)))
}

/// `html` under [`normal`], and then as the rule writes a URL that fails:
/// a link with no `href` as its text, and an image with no `src` not at all.
///
/// The spec writes a link with any scheme, and the sanitizer drops the
/// `href` of one it refuses. The renderer writes that link as its text.
fn spec_normal(html: &str) -> String {
    let mut tree = rebuilt(&parse(html));
    let mut work = vec![&mut tree];
    while let Some(node) = work.pop() {
        let has = |node: &Parsed, name: &str| node.attributes.iter().any(|(n, _)| n == name);
        let mut children = Vec::with_capacity(node.children.len());
        for child in std::mem::take(&mut node.children) {
            let element = child.kind == Kind::Element;
            if element && child.name == "img" && !has(&child, "src") {
                continue;
            }
            if element && child.name == "a" && !has(&child, "href") {
                children.extend(child.children);
            } else {
                children.push(child);
            }
        }
        node.children = children;
        work.extend(node.children.iter_mut());
    }
    normal(&source(&tree))
}

/// The examples among `examples` whose output differs from the spec's, each
/// as one line of the report.
fn failures(examples: &[Example], expected: impl Fn(&Example) -> Option<String>) -> Vec<String> {
    examples
        .iter()
        .filter_map(|example| {
            let want = expected(example)?;
            let got = rendered(&example.markdown, "a test").expect("no limit in a test");
            (spec_normal(&got) != spec_normal(&want)).then(|| {
                format!(
                    "example {}: {:?}\n  want {:?}\n  got  {:?}",
                    example.number, example.markdown, want, got
                )
            })
        })
        .collect()
}

#[test]
#[cfg_attr(
    miri,
    ignore = "reads the spec file, which Miri's isolation does not allow"
)]
fn every_commonmark_spec_example_renders_as_the_spec_says() {
    let commonmark = examples("commonmark-0.31.2.txt");
    let read = commonmark
        .iter()
        .filter(|example| autolinked(example).is_some());
    assert_eq!(
        read.count(),
        5,
        "five examples are read with the autolink extension on"
    );
    let report = failures(&commonmark, |example| {
        Some(autolinked(example).unwrap_or_else(|| example.html.clone()))
    });
    assert!(
        report.is_empty(),
        "{} failure(s):\n{}",
        report.len(),
        report.join("\n")
    );
}

/// The five CommonMark examples that write a bare URL or email address as
/// text, with their HTML when GFM's extended autolinks are on.
///
/// Both spec files hold them, and each expects the extension off.
/// `rule:core-classes/markdown-to-html` reads one flavour with the extension
/// on, so each is a link, as cmark-gfm writes it with `-e autolink`.
const AUTOLINKED: &[(&str, &str)] = &[
    (
        "<https://foo.bar/baz bim>\n",
        "<p>&lt;<a href=\"https://foo.bar/baz\">https://foo.bar/baz</a> bim&gt;</p>\n",
    ),
    (
        "<http://foo.bar/baz bim>\n",
        "<p>&lt;<a href=\"http://foo.bar/baz\">http://foo.bar/baz</a> bim&gt;</p>\n",
    ),
    (
        "<foo\\+@bar.example.com>\n",
        "<p>&lt;<a href=\"mailto:foo+@bar.example.com\">foo+@bar.example.com</a>&gt;</p>\n",
    ),
    (
        "< https://foo.bar >\n",
        "<p>&lt; <a href=\"https://foo.bar\">https://foo.bar</a> &gt;</p>\n",
    ),
    (
        "< http://foo.bar >\n",
        "<p>&lt; <a href=\"http://foo.bar\">http://foo.bar</a> &gt;</p>\n",
    ),
    (
        "https://example.com\n",
        "<p><a href=\"https://example.com\">https://example.com</a></p>\n",
    ),
    (
        "http://example.com\n",
        "<p><a href=\"http://example.com\">http://example.com</a></p>\n",
    ),
    (
        "foo@bar.example.com\n",
        "<p><a href=\"mailto:foo@bar.example.com\">foo@bar.example.com</a></p>\n",
    ),
];

/// [`AUTOLINKED`]'s HTML for `example`, or `None` when it is not one of them.
fn autolinked(example: &Example) -> Option<String> {
    AUTOLINKED
        .iter()
        .find(|(markdown, _)| *markdown == example.markdown)
        .map(|(_, html)| (*html).to_owned())
}

/// The GFM examples whose emphasis predates CommonMark 0.31.2's delimiter
/// rule. `rule:core-classes/markdown-to-html` takes CommonMark's output for
/// each.
const EMPHASIS_BEFORE_0_31_2: &[usize] = &[398, 426, 434, 435, 436, 473, 474, 475, 477];

/// The one example of GFM's tag filter, which the rule does not apply.
const TAG_FILTER: usize = 652;

#[test]
#[cfg_attr(
    miri,
    ignore = "reads the spec file, which Miri's isolation does not allow"
)]
fn every_gfm_spec_example_renders_as_the_spec_says() {
    let commonmark = examples("commonmark-0.31.2.txt");
    let gfm = examples("gfm-0.29.txt");
    assert_eq!(gfm.len(), 672, "the vendored GFM spec has 672 examples");
    assert!(gfm[TAG_FILTER - 1].header.contains("tagfilter"));
    let report = failures(&gfm, |example| {
        if example.header.contains("disabled") || example.number == TAG_FILTER {
            return None;
        }
        if let Some(html) = autolinked(example) {
            return Some(html);
        }
        if EMPHASIS_BEFORE_0_31_2.contains(&example.number) {
            let same = commonmark
                .iter()
                .find(|other| other.markdown == example.markdown)
                .unwrap_or_else(|| panic!("example {} is in CommonMark's spec", example.number));
            return Some(same.html.clone());
        }
        Some(example.html.clone())
    });
    assert!(
        report.is_empty(),
        "{} failure(s):\n{}",
        report.len(),
        report.join("\n")
    );
}

#[test]
#[cfg_attr(
    miri,
    ignore = "reads the spec files, which Miri's isolation does not allow"
)]
fn sanitizing_the_output_again_changes_nothing() {
    let mut inputs: Vec<String> = examples("commonmark-0.31.2.txt")
        .into_iter()
        .chain(examples("gfm-0.29.txt"))
        .map(|example| example.markdown)
        .collect();
    inputs.extend(
        [
            "[a](javascript:alert(1)) ![b](data:text/html,x) [c](//evil.example)",
            "[a](java%73cript:x) [b](%2F%2Fevil.example) [c](/ok%20path)",
            "x[^1] y[^2] z[^1]\n\n[^1]: one\n\n[^2]: - two\n\n[^3]: unused\n",
            "```language-r\nx\n```\n\n```;\ny\n```\n",
            "a\0b\r\nc\rd\u{A0}e \"q\" 'r'",
            "\u{FEFF}# heading",
            "- [ ] open\n- [x] done\n",
            "| a | b |\n|:-|-:|\n| 1 |\n",
        ]
        .map(str::to_owned),
    );
    let report: Vec<String> = inputs
        .iter()
        .filter_map(|markdown| {
            let out = rendered(markdown, "a test").expect("no limit in a test");
            let again = normal(&out);
            (again != out).then(|| format!("{markdown:?}\n  out   {out:?}\n  again {again:?}"))
        })
        .collect();
    assert!(
        report.is_empty(),
        "{} input(s):\n{}",
        report.len(),
        report.join("\n")
    );
}

/// The depth of the deepest element in `tree`.
fn depth(tree: &Parsed) -> usize {
    let mut deepest = 0;
    let mut work = vec![(tree, 0)];
    while let Some((node, at)) = work.pop() {
        deepest = deepest.max(at);
        work.extend(
            node.children
                .iter()
                .filter(|child| child.kind == Kind::Element)
                .map(|child| (child, at + 1)),
        );
    }
    deepest
}

#[test]
fn nesting_past_512_flattens_as_the_html_parser_does() {
    for markdown in [
        format!("{}a\n", "> ".repeat(OPEN_ELEMENTS_CAP * 2)),
        format!("{}a\n", "* ".repeat(OPEN_ELEMENTS_CAP)),
        format!(
            "{}a{}",
            "*a **".repeat(OPEN_ELEMENTS_CAP),
            "** a*".repeat(OPEN_ELEMENTS_CAP)
        ),
    ] {
        let (unflattened, _) = tree(&markdown);
        assert!(
            depth(&unflattened) > OPEN_ELEMENTS_CAP,
            "the input nests past the cap"
        );
        let want = normal(&source(&unflattened));
        let got = rendered(&markdown, "a test").expect("no limit in a test");
        assert_eq!(got, want, "the flatten is the parser's");
        assert!(depth(&parse(&got)) <= OPEN_ELEMENTS_CAP);
        assert_eq!(normal(&got), got, "the flattened output is a fixed point");
    }
}

#[test]
fn added_output_past_its_budget_renders_as_source_text() {
    let url = format!("/{}", "u".repeat(1000));
    let uses = (ADDED_OUTPUT_SLACK / url.len()) * 4;
    let markdown = format!("[x]: {url}\n\n{}", "[x] ".repeat(uses));
    let out = rendered(&markdown, "a test").expect("no limit in a test");
    let links = out.matches("<a href=").count();
    assert!(links > 0, "a reference inside the budget is a link");
    assert!(links < uses, "a reference past the budget is not");
    assert!(
        out.contains("[x] [x]"),
        "a reference past the budget is its source text"
    );
    assert!(
        out.len() <= 2 * markdown.len() + ADDED_OUTPUT_SLACK,
        "the output is {} bytes for {} bytes of input",
        out.len(),
        markdown.len()
    );

    let columns = 64;
    let rows = 4096;
    let markdown = format!(
        "{}|\n{}|\n{}",
        "|x".repeat(columns),
        "|-".repeat(columns),
        "a\n".repeat(rows)
    );
    let out = rendered(&markdown, "a test").expect("no limit in a test");
    let padded = rows * (columns - 1) * PADDED_CELL;
    assert!(
        padded > markdown.len() + ADDED_OUTPUT_SLACK,
        "the table pads past the budget"
    );
    let head = columns * "<th>x</th>\n".len()
        + "<table>\n<thead>\n<tr>\n</tr>\n</thead>\n<tbody>\n</tbody>\n</table>\n".len();
    assert!(
        out.len()
            <= markdown.len()
                + ADDED_OUTPUT_SLACK
                + head
                + rows * "<tr>\n<td>a</td>\n</tr>\n".len(),
        "a row past the budget is not padded: {} bytes",
        out.len()
    );
}

/// Every `work = "linear"` to-html entry of the known-bug catalogue, as its
/// id and its input with `repeat` written `n` times.
fn slow_inputs(n: usize) -> Vec<(String, String)> {
    let path = format!(
        "{}/../../tests/data/markdown/known-bugs.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let catalogue: toml::Table = toml::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
    let field = |bug: &toml::Table, key: &str| {
        bug.get(key)
            .and_then(toml::Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    catalogue["bug"]
        .as_array()
        .expect("`bug` is an array of tables")
        .iter()
        .filter_map(toml::Value::as_table)
        .filter(|bug| field(bug, "work") == "linear" && field(bug, "direction") == "to-html")
        .map(|bug| {
            let text =
                field(bug, "input") + &field(bug, "repeat").repeat(n) + &field(bug, "suffix");
            (field(bug, "id"), text)
        })
        .collect()
}

/// `rule:core-classes/markdown-cost`'s linear work, over every catalogued
/// slow input.
///
/// Each input renders with its `repeat` written `n` times and then `4n`
/// times, counting the engine's steps and the bytes written
/// ([`crate::fuzz::markdown_counting_steps`]). Linear work takes about four
/// times the steps at `4n`. Work in the square of the input takes sixteen.
/// The bound is five, which leaves room for the steps that do not grow.
#[test]
#[cfg_attr(miri, ignore = "hundreds of documents of thousands of lines")]
#[cfg_attr(
    not(debug_assertions),
    ignore = "the engine counts its steps only with debug assertions"
)]
fn work_grows_linearly_over_every_catalogued_slow_input() {
    const N: usize = 1000;
    let steps = |text: &str| {
        crate::fuzz::markdown_counting_steps(text)
            .1
            .expect("a build with debug assertions counts steps")
    };
    let short = slow_inputs(N);
    let long = slow_inputs(4 * N);
    assert!(short.len() > 100, "the catalogue has its slow entries");
    let report: Vec<String> = short
        .iter()
        .zip(&long)
        .filter_map(|((id, short), (_, long))| {
            let (small, large) = (steps(short), steps(long));
            // A longer input that costs no more steps would mean the count
            // misses the loop the input grows, which would hide a slow one.
            (large <= small || large > 5 * small)
                .then(|| format!("{id}: {small} steps at n = {N}, {large} at 4n"))
        })
        .collect();
    assert!(
        report.is_empty(),
        "{} entries grow faster than their input:\n{}",
        report.len(),
        report.join("\n")
    );
}

type UrlHook = Box<dyn Fn(&str) -> Answer>;
type HeadingHook = Box<dyn Fn(usize, &str) -> Answer>;
type CodeHook = Box<dyn Fn(&str, &str) -> Answer>;

/// The four callbacks as Rust closures, each `None` when the test passes none,
/// and what each was called with.
#[derive(Default)]
struct Hooks {
    link: Option<UrlHook>,
    image: Option<UrlHook>,
    heading: Option<HeadingHook>,
    code: Option<CodeHook>,
    calls: Vec<String>,
}

impl Callbacks for Hooks {
    fn link(&mut self, url: &str) -> Result<Answer, Fault> {
        self.calls.push(format!("link {url}"));
        Ok(self.link.as_ref().map_or(Answer::Default, |link| link(url)))
    }

    fn image(&mut self, url: &str) -> Result<Answer, Fault> {
        self.calls.push(format!("image {url}"));
        Ok(self
            .image
            .as_ref()
            .map_or(Answer::Default, |image| image(url)))
    }

    fn wants_heading(&self) -> bool {
        self.heading.is_some()
    }

    fn heading(&mut self, level: usize, text: &str) -> Result<Answer, Fault> {
        self.calls.push(format!("heading {level} {text}"));
        Ok(self
            .heading
            .as_ref()
            .map_or(Answer::Default, |heading| heading(level, text)))
    }

    fn wants_code(&self) -> bool {
        self.code.is_some()
    }

    fn code(&mut self, language: &str, code: &str) -> Result<Answer, Fault> {
        self.calls.push(format!("code {language} {code:?}"));
        Ok(self
            .code
            .as_ref()
            .map_or(Answer::Default, |hook| hook(language, code)))
    }
}

/// A URL callback that returns `url` whatever it is given.
fn returning(url: &'static str) -> Option<UrlHook> {
    Some(Box::new(move |_| Answer::Value(url.to_owned())))
}

/// `markdown` rendered under `hooks`.
fn rendered_by(markdown: &str, hooks: &mut Hooks) -> String {
    rendered_with(markdown, "a test", hooks).expect("no limit in a test")
}

#[test]
fn a_link_callback_cannot_produce_a_javascript_url() {
    let markdown = "[a](https://example.com/a) and www.example.com and <https://example.com/b>\n";
    let mut hooks = Hooks {
        link: returning("java\tscript:alert(1)"),
        ..Hooks::default()
    };
    let out = rendered_by(markdown, &mut hooks);
    assert_eq!(
        hooks.calls,
        [
            "link https://example.com/a",
            "link http://www.example.com",
            "link https://example.com/b"
        ]
    );
    assert_eq!(
        out, "<p>a and www.example.com and https://example.com/b</p>\n",
        "every link is written as its text"
    );

    for refused in [
        "javascript:alert(1)",
        "%6Aavascript:alert(1)",
        "//example.org/x",
    ] {
        let mut hooks = Hooks {
            link: returning(refused),
            ..Hooks::default()
        };
        let out = rendered_by("[a](https://example.com/)\n", &mut hooks);
        assert_eq!(out, "<p>a</p>\n", "{refused} is not written");
    }

    let mut hooks = Hooks {
        link: Some(Box::new(|url| {
            Answer::Value(url.replace("example.com", "example.org"))
        })),
        ..Hooks::default()
    };
    assert_eq!(
        rendered_by("[a](https://example.com/x)\n", &mut hooks),
        "<p><a href=\"https://example.org/x\">a</a></p>\n"
    );
    let mut hooks = Hooks {
        link: Some(Box::new(|_| Answer::Null)),
        ..Hooks::default()
    };
    assert_eq!(
        rendered_by("[a *b*](https://example.com/x)\n", &mut hooks),
        "<p>a <em>b</em></p>\n",
        "`null` keeps the link's text"
    );
}

#[test]
fn an_image_callback_returning_null_removes_the_image() {
    let mut hooks = Hooks {
        image: Some(Box::new(|_| Answer::Null)),
        ..Hooks::default()
    };
    let out = rendered_by("a ![a cat](https://example.com/cat.png) b\n", &mut hooks);
    assert_eq!(hooks.calls, ["image https://example.com/cat.png"]);
    assert_eq!(out, "<p>a  b</p>\n");

    let mut hooks = Hooks {
        image: Some(Box::new(|url| {
            Answer::Value(format!("https://cdn.example.com/{url}"))
        })),
        ..Hooks::default()
    };
    assert_eq!(
        rendered_by("![a cat](cat.png)\n", &mut hooks),
        "<p><img src=\"https://cdn.example.com/cat.png\" alt=\"a cat\"></p>\n"
    );

    let mut hooks = Hooks {
        image: returning("javascript:alert(1)"),
        ..Hooks::default()
    };
    assert_eq!(
        rendered_by("![a cat](cat.png)\n", &mut hooks),
        "<p></p>\n",
        "a refused URL removes the image"
    );
}

#[test]
fn a_heading_callback_sets_the_id() {
    let mut hooks = Hooks {
        heading: Some(Box::new(|_, text| {
            Answer::Value(text.to_lowercase().replace(' ', "-"))
        })),
        ..Hooks::default()
    };
    let out = rendered_by("## Hello *new* `World`\n\nSetext\n===\n", &mut hooks);
    assert_eq!(
        hooks.calls,
        ["heading 2 Hello new World", "heading 1 Setext"]
    );
    assert_eq!(
        out,
        "<h2 id=\"md-hello-new-world\">Hello <em>new</em> <code>World</code></h2>\n\
         <h1 id=\"md-setext\">Setext</h1>\n"
    );

    for answer in [None, Some("a b"), Some("")] {
        let mut hooks = Hooks {
            heading: Some(Box::new(move |_, _| {
                answer.map_or(Answer::Null, |id| Answer::Value(id.to_owned()))
            })),
            ..Hooks::default()
        };
        assert_eq!(
            rendered_by("# A\n", &mut hooks),
            "<h1>A</h1>\n",
            "{answer:?} writes no `id`"
        );
    }

    let mut hooks = Hooks {
        heading: Some(Box::new(|_, _| {
            Answer::Value("\"><script>alert(1)</script>".to_owned())
        })),
        ..Hooks::default()
    };
    assert_eq!(
        rendered_by("# A\n\n<div>raw</div>\n", &mut hooks),
        "<h1 id=\"md-&quot;><script>alert(1)</script>\">A</h1>\n<div>raw</div>\n",
        "the `id` stays one attribute value through the rebuild"
    );
}

#[test]
fn a_code_callback_replaces_a_fenced_block() {
    const HIGHLIGHTED: &str =
        "<pre class=\"hl\"><span style=\"color:red\">fn</span> main() {}</pre>";
    let mut hooks = Hooks {
        code: Some(Box::new(|language, _| {
            if language == "rust" {
                Answer::Value(HIGHLIGHTED.to_owned())
            } else {
                Answer::Null
            }
        })),
        ..Hooks::default()
    };
    let markdown =
        "```rust\nfn main() {}\n```\n\n```text\nplain\n```\n\n    indented\n\n```\nbare\n```\n";
    let out = rendered_by(markdown, &mut hooks);
    assert_eq!(
        hooks.calls,
        ["code rust \"fn main() {}\\n\"", "code text \"plain\\n\""],
        "only a fenced block that names a language reaches the callback"
    );
    assert_eq!(
        out,
        format!(
            "{HIGHLIGHTED}\n<pre><code class=\"language-text\">plain\n</code></pre>\n\
             <pre><code>indented\n</code></pre>\n<pre><code>bare\n</code></pre>\n"
        ),
        "`null` keeps the default block"
    );

    // Raw HTML elsewhere sends the document through the sanitizer's rebuild,
    // which would remove the `style`. The `Markup` is written as it is.
    let out = rendered_by(
        "<div onclick=\"x\">raw</div>\n\n```rust\nfn main() {}\n```\n",
        &mut hooks,
    );
    assert!(
        out.contains(HIGHLIGHTED),
        "the fragment is unchanged: {out}"
    );
    assert!(!out.contains("onclick"), "the document is rebuilt: {out}");
    assert!(!out.contains('\u{E000}'), "no token is left: {out}");
}

/// `markdown` written one block group at a time, read in one piece.
fn grouped(markdown: &str) -> Vec<String> {
    groups::grouped_with(markdown, "a test", &mut NoCallbacks).expect("no limit in a test")
}

/// `markdown` written one block group at a time, read `piece` bytes at a time.
fn grouped_in(markdown: &str, piece: usize) -> Vec<String> {
    segments::grouped_in(markdown.as_bytes(), piece, "a test", &mut NoCallbacks)
        .expect("no limit in a test")
}

/// Every to-html entry of the known-bug catalogue, as its id and its input
/// with `repeat` written `n` times.
fn catalogue_inputs(n: usize) -> Vec<(String, String)> {
    let path = format!(
        "{}/../../tests/data/markdown/known-bugs.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let catalogue: toml::Table = toml::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
    let field = |bug: &toml::Table, key: &str| {
        bug.get(key)
            .and_then(toml::Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    catalogue["bug"]
        .as_array()
        .expect("`bug` is an array of tables")
        .iter()
        .filter_map(toml::Value::as_table)
        .filter(|bug| field(bug, "direction") == "to-html")
        .map(|bug| {
            let text =
                field(bug, "input") + &field(bug, "repeat").repeat(n) + &field(bug, "suffix");
            (field(bug, "id"), text)
        })
        .collect()
}

/// `rule:core-classes/markdown-chunks`' acceptance: the groups of every spec
/// example and every catalogued input join to the whole document's HTML.
#[test]
#[cfg_attr(
    miri,
    ignore = "reads the spec files, which Miri's isolation does not allow"
)]
fn rendering_in_chunks_gives_the_same_html_as_the_whole_document() {
    let mut inputs: Vec<(String, String)> = examples("commonmark-0.31.2.txt")
        .into_iter()
        .map(|example| (format!("commonmark {}", example.number), example.markdown))
        .chain(
            examples("gfm-0.29.txt")
                .into_iter()
                .map(|example| (format!("gfm {}", example.number), example.markdown)),
        )
        .collect();
    inputs.extend(catalogue_inputs(64));
    inputs.extend(
        [
            "a[^1]\n\n<div>\n\nb\n\n</div>\n\n[^1]: <span>note</span>\n\nc [x]\n\n[x]: /u\n",
            "<!DOCTYPE html>\n<p>a\n\n# b\n\n<table><tr><td>c</td></tr></table>\n",
            "# a\n\n  <!-- c -->\n\n<p>x <table>\n\n- b\n- c\n\n<script>\n\nd\n</script>\n\ne\n",
            "x <span a=\"1>2\">y\n\nz</span>\n\nw <!-- open\n\nq -->\n\nend\n",
        ]
        .map(|text| ("inline".to_owned(), text.to_owned())),
    );
    // Each spec file whole is one more document, with forward references
    // and footnotes across many segments.
    for file in ["commonmark-0.31.2.txt", "gfm-0.29.txt"] {
        let joined: Vec<String> = examples(file).into_iter().map(|e| e.markdown).collect();
        inputs.push((format!("{file} joined"), joined.join("\n")));
    }
    let mut split = 0;
    let report: Vec<String> = inputs
        .iter()
        .flat_map(|(id, markdown)| {
            let whole = rendered(markdown, "a test").expect("no limit in a test");
            let groups = grouped(markdown);
            split += usize::from(groups.len() > 1);
            // A piece of 1 byte cuts a segment wherever one can be cut, and
            // 7 bytes splits UTF-8 sequences and CR LF pairs elsewhere.
            [groups, grouped_in(markdown, 1), grouped_in(markdown, 7)]
                .into_iter()
                .enumerate()
                .filter_map(move |(read, groups)| {
                    let joined = groups.concat();
                    (joined != whole).then(|| {
                        format!(
                            "{id} read {read} {markdown:?}\n  whole  {whole:?}\n  groups {groups:?}"
                        )
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        report.is_empty(),
        "{} input(s):\n{}",
        report.len(),
        report.join("\n")
    );
    assert!(split > 100, "only {split} inputs had more than one group");
}

/// A raw HTML element that is still open keeps the blocks after it in its
/// group, up to the block that closes it.
#[test]
fn an_unclosed_raw_html_element_keeps_its_blocks_in_one_group() {
    let groups = grouped("<div>\n\n# a\n\npara\n\n</div>\n\nafter\n");
    assert_eq!(groups.len(), 2, "{groups:?}");
    assert!(
        groups[0].contains("<h1>a</h1>") && groups[0].contains("</div>"),
        "{groups:?}"
    );
    assert_eq!(groups[1], "<p>after</p>\n");

    let groups = grouped("a <span>b\n\nc\n\nd</span>\n\ne\n");
    assert_eq!(groups.len(), 2, "{groups:?}");
    assert!(groups[0].contains("<p>c</p>"), "{groups:?}");

    let groups = grouped("# a\n\nb\n\n- c\n");
    assert_eq!(
        groups.len(),
        3,
        "a document with no raw HTML ends a group at each block: {groups:?}"
    );
}

/// `rule:core-classes/markdown-chunks`' memory: a document read forward in
/// reads, as a file is, holds a few reads, one group and the definitions at
/// any point, however long it is. A reference defined at the end still
/// resolves in the first group.
#[test]
#[cfg_attr(
    miri,
    ignore = "renders 64 reads of text twice, which Miri takes minutes for; the walk is safe Rust"
)]
fn to_html_each_holds_one_block_group_of_a_file_not_the_document() {
    use crate::byte_source::READ_CHUNK;
    let paragraph = format!("{}[x]\n\n", "word ".repeat(40));
    let count = 64 * READ_CHUNK / paragraph.len();
    let mut markdown = paragraph.repeat(count);
    markdown.push_str("[x]: /target\n");
    let read = || segments::feed_bytes(markdown.as_bytes(), READ_CHUNK, "a test");
    let (outside, len) = segments::definitions(&mut read()).expect("text");
    let mut walk = segments::Walk::new(outside, len, "a test");
    let mut source = segments::Segments::default();
    let mut feed = read();
    let (mut most, mut groups, mut first) = (0, 0, String::new());
    while let Some(segment) = segments::next_segment(&mut source, &mut feed).expect("text") {
        walk.segment(&segment, &mut NoCallbacks)
            .expect("no callbacks");
        most = most.max(source.held() + segment.capacity() + walk.held());
        while let Some(group) = walk.pop() {
            groups += 1;
            if first.is_empty() {
                first = group;
            }
        }
    }
    assert!(groups >= count, "{groups} groups of {count} paragraphs");
    assert!(
        first.contains("<a href=\"/target\">x</a>"),
        "the reference resolves before its definition is read: {first}"
    );
    // The text is parsed again only once it has doubled, and a string's
    // capacity can be twice its length, so a segment is up to four reads.
    // The groups it writes are about as long again.
    let bound = 8 * READ_CHUNK;
    assert!(
        most <= bound,
        "held {most} bytes of a {} byte document, past {bound}",
        markdown.len()
    );
}
