//! `rule:core-classes/markdown-to-html`'s and `markdown-cost`'s properties
//! over the renderer, with the spec examples vendored under
//! `tests/data/markdown/spec/`.

use super::*;
use crate::html::{parse, parse_in_page, rebuilt, source};

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
    let path = nvs_repo::path(&format!("tests/data/markdown/spec/{name}"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
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
    source(&rebuilt(&parse_in_page(html)))
}

/// `html` under [`normal`], and then as the rule writes a URL that fails:
/// a link with no `href` as its text, and an image with no `src` not at all.
///
/// The spec writes a link with any scheme, and the sanitizer drops the
/// `href` of one it refuses. The renderer writes that link as its text.
fn spec_normal(html: &str) -> String {
    let mut tree = rebuilt(&parse_in_page(html));
    let mut work = vec![&mut tree];
    while let Some(node) = work.pop() {
        let has = |node: &Parsed, name: &str| node.attributes.iter().any(|(n, _)| n == name);
        let mut children = Vec::with_capacity(node.children.len());
        for mut child in std::mem::take(&mut node.children) {
            let element = child.kind == Kind::Element;
            if element && child.name == "img" && !has(&child, "src") {
                continue;
            }
            if element && child.name == "a" && !has(&child, "href") {
                children.append(&mut child.children);
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

// covers: Core\Markdown::toHtml
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
            // Raw `<table>` in a paragraph: in quirks mode the sanitizer's
            // answer would be a `p` inside a `p`.
            "a<table>",
            "<!DOCTYPE x>\n\na<table>",
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
    // Each shape is six HTML parses and renders, which Miri takes about
    // twenty seconds each for, so under Miri only the first shape runs.
    let shapes = if cfg!(miri) { 1 } else { 3 };
    for markdown in [
        format!("{}a\n", "> ".repeat(OPEN_ELEMENTS_CAP * 2)),
        format!("{}a\n", "* ".repeat(OPEN_ELEMENTS_CAP)),
        format!(
            "{}a{}",
            "*a **".repeat(OPEN_ELEMENTS_CAP),
            "** a*".repeat(OPEN_ELEMENTS_CAP)
        ),
    ]
    .into_iter()
    .take(shapes)
    {
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
#[cfg_attr(
    miri,
    ignore = "the budget is a fixed 64 KiB, so the documents are hundreds of kilobytes"
)]
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

/// Every `work = "linear"` entry of the known-bug catalogue whose direction
/// is `direction`, as its id and its input with `repeat` written `n` times.
fn slow_inputs(n: usize, direction: &str) -> Vec<(String, String)> {
    let path = nvs_repo::path("tests/data/markdown/known-bugs.toml")
        .display()
        .to_string();
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
        .filter(|bug| field(bug, "work") == "linear" && field(bug, "direction") == direction)
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
    let short = slow_inputs(N, "to-html");
    let long = slow_inputs(4 * N, "to-html");
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

/// A run of raw tags that never close is our own find, not another
/// library's, so it is here and not in the catalogue. `groups::Balance` once
/// scanned its open elements per start tag, which made 120,000 `<div>` take
/// seconds. The steps count the engine and the output; the open stack itself
/// is held linear by `Balance` reading a count of its foreign elements.
#[test]
fn a_run_of_unclosed_raw_tags_costs_linear_work() {
    // The steps are counted, not timed, so a short run under Miri still
    // tells linear work from square work, inside Miri's 300 s limit.
    const N: usize = if cfg!(miri) { 8 } else { 1000 };
    let steps = |text: &str| {
        crate::fuzz::markdown_counting_steps(text)
            .1
            .expect("a build with debug assertions counts steps")
    };
    for tag in ["<a>", "<div>", "<p><span>", "<svg>"] {
        let (small, large) = (steps(&tag.repeat(N)), steps(&tag.repeat(4 * N)));
        assert!(
            large > small && large <= 5 * small,
            "{tag}: {small} steps at n = {N}, {large} at 4n"
        );
    }
    let mut balance = groups::Balance::default();
    balance.feed(&"<svg><math>".repeat(N));
    balance.feed(&"</math></svg>".repeat(N));
    assert!(
        balance.closed(),
        "every foreign element it opened is closed"
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
    let path = nvs_repo::path("tests/data/markdown/known-bugs.toml")
        .display()
        .to_string();
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
// covers: Core\Markdown::toHtmlEach
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
            // Two declarations the rebuild drops: the group after them is
            // still the start of the page, so its whitespace is dropped.
            "<!x\n>\n <!y\n>\n\na\n",
            // The list's range reaches into the line of the second `*`, which
            // continues the paragraph the definition starts.
            "*\n[a]: /u\n  *\n\nb\n",
            ">\n*\n[a]: /u\n*\nl\n\nb\n",
            "*\n[a]: /u\nx\nb |\n-:|\n\n[a]\n",
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

/// A raw-text HTML block, with or without a comment opener inside it, and a
/// block after it: read in small pieces, the groups join to the whole
/// document's HTML.
///
/// Under Miri it takes two of the four tags and three piece sizes, about an
/// eleventh of the work, so it ends inside the Miri profile's five minutes.
#[test]
fn a_raw_text_html_block_read_in_small_pieces_joins_to_the_whole_document() {
    let tags: &[&str] = if cfg!(miri) {
        &["script", "pre"]
    } else {
        &["script", "style", "pre", "textarea"]
    };
    let pieces: Vec<usize> = if cfg!(miri) {
        vec![1, 2, 7]
    } else {
        (1..=16).collect()
    };
    let mut report = Vec::new();
    for &tag in tags {
        let double = format!("<!--<{tag}>");
        let left = format!("<!--<{tag}></{tag}>-->");
        let back = format!("<!--<{tag} a></{tag}>");
        for inside in ["", "<!--", "<!-->", "<!-- x -->", &double, &left, &back] {
            for after in ["\n-", "\n- a\n- b\n", "\n\n# h\n", "\npara\n", "\n\n> q\n"] {
                for close in [String::new(), format!("</{tag}>")] {
                    let markdown = format!("<{tag}>{inside}{close}{after}");
                    let whole = rendered(&markdown, "a test").expect("no limit in a test");
                    if grouped(&markdown).concat() != whole {
                        report.push(format!("one piece {markdown:?}"));
                    }
                    for &piece in &pieces {
                        let groups = grouped_in(&markdown, piece);
                        if groups.concat() != whole {
                            report.push(format!(
                                "piece {piece} {markdown:?}\n  whole  {whole:?}\n  groups {groups:?}"
                            ));
                            break;
                        }
                    }
                }
            }
        }
    }
    let markdown = "<script><!--<script></script>\n-";
    let whole = rendered(markdown, "a test").expect("no limit in a test");
    for &piece in &pieces {
        assert_eq!(grouped_in(markdown, piece).concat(), whole, "piece {piece}");
    }
    assert!(
        report.is_empty(),
        "{} input(s):\n{}",
        report.len(),
        report.join("\n")
    );
}

/// `markdown` pushed into a `Core\Markdown\Live` `piece` bytes at a time:
/// the `done` part of each push, and what `finish()` returns.
fn live_in(markdown: &str, piece: usize) -> (Vec<String>, String) {
    let (parts, rest) = crate::fuzz::markdown_live(markdown, piece);
    (parts.into_iter().map(|(done, _open)| done).collect(), rest)
}

/// `rule:core-classes/markdown-chunks`' Live: over a document with no
/// reference used before its definition, the `done` parts of every push and
/// `finish()` joined are the HTML `toHtml` gives for the whole document,
/// whatever size the pieces are.
#[test]
#[cfg_attr(
    miri,
    ignore = "reads the spec files, which Miri's isolation does not allow"
)]
fn live_done_parts_and_finish_join_to_the_whole_document_html() {
    // A definition can be used only once it has arrived, so the corpus is
    // every spec example with no definition, and documents that define
    // before they use.
    let mut inputs: Vec<(String, String)> = examples("commonmark-0.31.2.txt")
        .into_iter()
        .map(|example| (format!("commonmark {}", example.number), example.markdown))
        .chain(
            examples("gfm-0.29.txt")
                .into_iter()
                .map(|example| (format!("gfm {}", example.number), example.markdown)),
        )
        .filter(|(_, markdown)| !markdown.contains("]:"))
        .collect();
    inputs.extend(
        [
            "[x]: /u \"t\"\n[^n]: a note\n\n# Title\n\nSee [x] and [y][x].[^n]\n\n- one\n- two\n\n  more\n\n```rust\nfn main() {}\n```\n\n| a | b |\n|---|:-:|\n| 1 | 2 |\n\nend\r\nline\r\n",
            "<div>\n\n# a\n\npara\n\n</div>\n\nafter <span>x\n\ny</span>\n\n> quote\n> more\n\n1. a\n2. b\n",
            "Setext\n======\n\n    indented\n    code\n\n* * *\n\n<!-- a\n\nb -->\n\nlast é 漢字 line",
            "- [ ] task\n- [x] done\n\n~~gone~~ www.example.com and https://example.com/a?b\n",
        ]
        .map(|text| ("inline".to_owned(), text.to_owned())),
    );
    let mut report = Vec::new();
    let mut finished_early = 0;
    for (id, markdown) in &inputs {
        let whole = rendered(markdown, "a test").expect("no limit in a test");
        for piece in 1..=markdown.len().max(1) {
            let (done, rest) = live_in(markdown, piece);
            finished_early += usize::from(done.iter().any(|part| !part.is_empty()));
            let joined = done.concat() + &rest;
            if joined != whole {
                report.push(format!(
                    "{id} piece {piece} {markdown:?}\n  whole {whole:?}\n  live  {joined:?}"
                ));
                break;
            }
        }
    }
    assert!(
        report.is_empty(),
        "{} input(s):\n{}",
        report.len(),
        report.join("\n")
    );
    assert!(
        finished_early > 1000,
        "only {finished_early} reads finished a part before `finish()`"
    );
}

/// `rule:core-classes/markdown-chunks`' Live: a finished part never changes.
/// A reference defined after it stays text there, though the whole document
/// links it, and every piece size gives the same parts.
// covers: Core\Markdown::live, Core\Markdown\Live::push, Core\Markdown\Live::finish, Core\Markdown\Update::done, Core\Markdown\Update::open
#[test]
fn live_never_changes_a_part_it_already_finished() {
    // Every finished part is an HTML render, which Miri takes about twenty
    // seconds for. Under Miri the first document runs, in pieces of one byte.
    let (documents, last_piece) = if cfg!(miri) { (1, 1) } else { (3, usize::MAX) };
    for markdown in [
        "Read [the guide].\n\nMore text.\n\n[the guide]: /guide\n",
        "a[^1] and [x][]\n\n# b\n\n[^1]: late note\n\n[x]: /late\n",
        "[x]: /first\n\n[x]\n\n[x]: /second\n\n[x]\n",
    ]
    .into_iter()
    .take(documents)
    {
        let whole = rendered(markdown, "a test").expect("no limit in a test");
        let (first, first_rest) = live_in(markdown, markdown.len());
        let first = first.concat() + &first_rest;
        for piece in 1..=markdown.len().min(last_piece) {
            let (done, rest) = live_in(markdown, piece);
            assert_eq!(
                done.concat() + &rest,
                first,
                "piece {piece} of {markdown:?}"
            );
        }
        if markdown.starts_with("[x]:") {
            assert_eq!(first, whole, "the first definition wins in both");
        } else {
            assert_ne!(first, whole, "{markdown:?} has a forward reference");
            assert!(
                first.contains("<p>Read [the guide].</p>")
                    || first.contains("<p>a[^1] and [x][]</p>"),
                "a part finished before the definition keeps it as text: {first}"
            );
        }
    }
}

/// `rule:core-classes/markdown-chunks`' memory: `toHtmlEach` over a
/// `Core\IO\File` reads the file twice and holds a few reads, one group and
/// the definitions at any point, however long the file is. A reference
/// defined at the end still resolves in the first group.
#[test]
#[cfg_attr(
    miri,
    ignore = "renders 64 reads of a file twice, which Miri takes minutes for and cannot open"
)]
fn to_html_each_holds_one_block_group_of_a_file_not_the_document() {
    use crate::byte_source::READ_CHUNK;
    use nvs_runtime::{Ctx, OutputSink};
    let paragraph = format!("{}[x]\n\n", "word ".repeat(40));
    let count = 64 * READ_CHUNK / paragraph.len();
    let mut markdown = paragraph.repeat(count);
    markdown.push_str("[x]: /target\n");
    let dir = nvs_repo::scratch("markdown-each");
    let path = dir.join("long.md");
    std::fs::write(&path, &markdown).expect("the case writes its file");

    let mut ctx = Ctx::new(OutputSink::Sink);
    let file = std::fs::File::open(&path).expect("the case opens its file");
    let key = ctx.hold_open_file(file);
    let handle = crate::io::file_handle(
        key,
        Value::text("long.md"),
        nvs_runtime::capability::Access::Read,
    );
    let null = Value::null();
    let groups_value = nvs_runtime::call(
        each::nvs_core_markdown_to_html_each,
        &mut ctx,
        &[handle, null, null, null, null],
    )
    .expect("an open file is a source");
    let (mut most, mut groups, mut first) = (0, 0, String::new());
    while let Some(group) = each::advanced(&mut ctx, groups_value).expect("the file reads") {
        let (held, rereads) = each::held_by(&mut ctx, groups_value).expect("the walk is open");
        assert!(rereads, "a file is read again, not held whole");
        most = most.max(held);
        groups += 1;
        if first.is_empty() {
            first = group;
        }
    }
    #[expect(
        unsafe_code,
        reason = "the case owns the one reference of each value it built"
    )]
    unsafe {
        groups_value.release();
        handle.release();
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

use crate::fuzz::html_meaning as meaning;

/// `text` as HTML text.
fn html_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Texts that are Markdown syntax where they stand, each one at the start of
/// a line.
const SYNTAX: &[&str] = &[
    "# a",
    "## a ##",
    "- a",
    "+ a",
    "* a",
    "1. a",
    "1984) a",
    "> a",
    ">a",
    "*a*",
    "**a**",
    "_a_",
    "~~a~~",
    "`a`",
    "```",
    "~~~",
    "---",
    "***",
    "===",
    "[a](https://example.com)",
    "![a](https://example.com/a.png)",
    "[a]: https://example.com",
    "[^1]",
    "<b>a</b>",
    "<!-- a -->",
    "&amp; &copy; &#65;",
    "\\*a\\*",
    "| a | b |",
    "www.example.com",
    "https://example.com/a",
    "a@example.com",
    "see http://a@example.com",
    "[ ] a",
    "a \\",
];

// covers: Core\Markdown::fromHtml
#[test]
#[cfg_attr(
    miri,
    ignore = "parses and renders over a hundred documents, which Miri takes about twenty seconds each for"
)]
fn from_html_text_never_becomes_markdown_syntax() {
    for text in SYNTAX {
        let body = html_text(text);
        for (html, want) in [
            (format!("<p>{body}</p>"), format!("<p>{body}</p>")),
            (format!("<h2>{body}</h2>"), format!("<h2>{body}</h2>")),
            (
                format!("<blockquote>{body}</blockquote>"),
                format!("<blockquote><p>{body}</p></blockquote>"),
            ),
            (
                format!("<ul><li>{body}</li></ul>"),
                format!("<ul><li>{body}</li></ul>"),
            ),
            (
                format!("<table><tr><th>{body}</th></tr></table>"),
                format!("<table><thead><tr><th>{body}</th></tr></thead></table>"),
            ),
            (format!("<p>x<br>{body}</p>"), format!("<p>x<br>{body}</p>")),
            (
                format!("<p><em>{body}</em></p>"),
                format!("<p><em>{body}</em></p>"),
            ),
        ] {
            let written = crate::fuzz::from_html(&html);
            let rendered = crate::fuzz::markdown(&written);
            assert_eq!(
                meaning(&rendered),
                meaning(&want),
                "{html:?} wrote {written:?}"
            );
        }
    }
}

/// Markdown documents whose HTML `fromHtml` writes back.
const DOCUMENTS: &[&str] = &[
    "# Title\n\nSome *emphasis*, **strong** and ~~struck~~ text.\n",
    "A [link](https://example.com/a \"its title\") and an ![image](/a.png \"t\").\n",
    "Code `a` and ``a ` b`` and ` `` `.\n",
    "```rust\nfn main() {}\n```\n\n```\n````\n```\n",
    "- one\n- two\n  - nested\n  - again\n- three\n",
    "3. three\n4. four\n\n- a\n\n- b\n",
    "- [ ] open\n- [x] done\n",
    "> quoted\n>\n> - a list\n> - in it\n",
    "| a | b | c |\n| :- | :-: | -: |\n| 1 | 2 | 3 |\n",
    "a\\\nb\n\n***\n\nc\n",
    "Visit www.example.com or https://example.com/x, or mail a@example.com.\n",
    "x<sup>2</sup> and H<sub>2</sub>O and <kbd>Ctrl</kbd>.\n",
    "1. a\n\n   b\n2. c\n",
    "> > deep\n> > quote\n",
    "- a\n\n  ```\n  code\n  ```\n- b\n",
    "### *a* `b` [c](/d)\n",
    "*a*b and a**b**c and *.a* and **(b)**\n",
    // A form feed is not CSS white space, so it stays in the text and in a
    // code span. The `markdown_from_html` fuzz target found the first.
    "\u{C}t\n",
    "a `\u{C}` b\n",
    // A break a browser shows comes back. The first was found by the
    // `markdown_from_html` fuzz target.
    "<br>\n",
    "<br>b\n\n# <br>c\n",
    "a<br><br>b and *c<br><br>*d\n",
    // A break inside struck text, followed by a line feed. A browser drops
    // the space at the start of the next line.
    "[~\\\n[~\\\n",
    // Two breaks at the start of emphasis, with a line feed between them.
    "*\\\n\\\n<**\n",
    // A link whose text ends with a space. `fromHtml` writes the space after
    // the link.
    "[\n\u{5} ](p=)b+\n",
    // A link whose text is a line feed alone.
    "a~[\n](+)\u{7}b\n",
    // A `.` three bytes after the start of a character. The
    // `markdown_from_html` fuzz target found that it panicked.
    "> \u{FFFD}!.\n",
    // Text that starts with U+FEFF. The `markdown_from_html` fuzz target found
    // that it was lost.
    "&#xFEFF;a\n",
    // A code span of one space. The `markdown_from_html` fuzz target found
    // that it was written as a plain space.
    "a ` ` b `\n` c\n",
    // A break at the start of emphasis is written in front of it. The
    // `markdown_from_html` fuzz target found it.
    "*\\\n*ww**\n",
    // Text after a nested list whose last item no line can continue stays in
    // a tight item. The `markdown_from_html` fuzz target found the first.
    "- *\n  *>\n",
    // A `!` right before a link. The `markdown_from_html` fuzz target found
    // that it made the link an image.
    "!o@example.com and \\![a](/b) and ![c](/d)\\![e](/f)\n",
    // A link whose text starts with a line feed, at the start of a
    // paragraph. A browser drops the space at the start of the line.
    "[\n\"]()b<\n",
    // Lists that each start an item. The `markdown_from_html` fuzz target
    // found that `fromHtml` wrote them as `- - -`, a thematic break.
    "*   -  *\n",
    "- a\n\n  -\n    -\n      - b\n",
    "- a\n  - # h\n  b\n- c\n",
    // Emphasis inside emphasis, with letters on both sides of the inner one.
    // The `markdown_from_html` fuzz target found that `fromHtml` wrote
    // `*E*EE*E*`, which reads as two separate emphases.
    "__E*EE*E_*zz<b_*zz<b\n<",
    "*a*b*c*d*",
    "**a**b**c**d**",
    "~~a~~b~~c~~d~~",
];

#[test]
#[cfg_attr(
    miri,
    ignore = "parses and renders dozens of documents, which Miri takes about twenty seconds each for"
)]
fn markdown_to_html_to_markdown_keeps_the_meaning() {
    for document in DOCUMENTS {
        let html = crate::fuzz::markdown(document);
        let written = crate::fuzz::from_html(&html);
        let again = crate::fuzz::markdown(&written);
        assert_eq!(
            meaning(&again),
            meaning(&html),
            "{document:?} rendered {html:?}, wrote {written:?}"
        );
    }
}

#[test]
#[cfg_attr(
    miri,
    ignore = "parses and renders dozens of documents, which Miri takes about twenty seconds each for"
)]
fn from_html_twice_gives_the_same_markdown() {
    let mut inputs: Vec<String> = DOCUMENTS
        .iter()
        .map(|document| crate::fuzz::markdown(document))
        .collect();
    inputs.extend(
        SYNTAX
            .iter()
            .map(|text| format!("<p>{}</p>", html_text(text))),
    );
    inputs.extend(
        [
            "<div><span>a</span> <b> b </b>c<br><br>d</div>",
            "<ol start=\"7\"><li><p>a</p></li><li>b</li></ol><ol><li>c</li></ol>",
            "<table><tr><td colspan=\"2\">a\n\nb</td></tr></table>",
            "<dl><dt>a</dt><dd>b</dd></dl>",
            "<pre>a\n\n  b</pre><p>x</p>",
            "<p><a href=\"javascript:x\">a</a> <img src=\"data:x\" alt=\"b\"></p>",
            // An empty list writes nothing. The `markdown_from_html` fuzz
            // target found that it added blank lines.
            "a<ul></ul><ol>\n</ol>",
            // One break alone at the end of a block with text is dropped.
            "<p>a<br></p><p>b<br><br></p><h2>c<br></h2>",
            // Text a Markdown reader would read as an email autolink. The
            // `markdown_from_html` fuzz target found the first.
            "<p>w@.wft and a@-b.c and d@_e.f</p>",
            // A line that starts with `:-` after a break. The
            // `markdown_from_html` fuzz target found that it was read as a
            // table's delimiter row.
            "`<br>:-",
            // URLs the renderer percent-encodes, and one with an entity's text.
            // The `markdown_from_html` fuzz target found the first.
            "<a href=\"<t\"></a><a href=\"/Straße\">a</a><a href=\"a b\">b</a>\
             <a href=\"a\\b\">c</a><a href=\"a&amp;amp;b\">d</a><a href=\"a(b\">e</a>\
             <img src=\"x y\" alt=\"f\">",
            // Two lists in a row that would each start with `- - -`.
            "<ul><li><ul><li><ul><li></li></ul></li></ul></li></ul>\
             <ul><li><ul><li><ul><li></li></ul></li></ul></li></ul>",
            // A list that starts with an empty item, after text in the same
            // item. The `markdown_from_html` fuzz target found the first:
            // `- a\n  -` reads as a heading.
            "<ul><)ulmmm><ul><ul>",
            "<ul><li>a<ul><li></li></ul></li><li>b</li></ul>",
            "<ul><li>a<ul><li></li><li>b</li></ul></li></ul>",
            "<ol><li>a<ol><li></li></ol></li><li>b</li></ol>",
            "<ul><li>a<table><tr><th>x</th></tr></table></li><li>b</li></ul>",
            // An empty `title`, which the HTML render does not write. The
            // fuzz target found the first.
            "**<a href=\"/u\" title=\"\"t>>*",
            "<img src=\"/i.png\" alt=\"x\" title=\"\">",
            // A checkbox outside a task list item, which writes nothing, at
            // the end of a block. The fuzz target found the first.
            "#  <input type=\"checkbox\" disabled checked>     \u{0}\u{0}\u{0}\u{0}\u{0} ",
            "<p>a <input type=\"checkbox\"> </p>",
        ]
        .map(str::to_owned),
    );
    for html in &inputs {
        let first = crate::fuzz::from_html(html);
        let second = crate::fuzz::from_html(&crate::fuzz::markdown(&first));
        assert_eq!(second, first, "{html:?}");
    }
}

#[test]
fn a_table_markdown_cannot_hold_is_written_as_sanitized_html() {
    let gfm = crate::fuzz::from_html("<table><tr><th>a</th></tr><tr><td>b</td></tr></table>");
    assert_eq!(gfm, "| a |\n| --- |\n| b |\n");

    // Each table is several HTML parses and renders, which Miri takes about
    // twenty seconds each for, so under Miri the first two tables run.
    let tables = if cfg!(miri) { 2 } else { usize::MAX };
    for html in [
        "<table><tr><th colspan=\"2\">a</th></tr><tr><td>b</td><td onclick=\"x()\">c</td></tr></table>",
        "<table><tr><th>a</th></tr><tr><td rowspan=\"2\">b</td></tr></table>",
        "<table><tr><th>a</th></tr><tr><td><p>b</p><p>c</p></td></tr></table>",
        "<table><caption>t</caption><tr><th>a</th></tr></table>",
        "<table><tr><td>no header</td></tr></table>",
        "<table><tr><th>a</th></tr><tr><td>b</td><td>c</td></tr></table>",
    ]
    .into_iter()
    .take(tables)
    {
        let written = crate::fuzz::from_html(html);
        assert!(written.starts_with("<table>"), "{html:?} wrote {written:?}");
        assert_eq!(written.lines().count(), 1, "one HTML block: {written:?}");
        assert!(!written.contains("onclick"), "{written:?}");
        assert_eq!(
            meaning(&crate::fuzz::markdown(&written)),
            meaning(html),
            "{html:?} wrote {written:?}"
        );
    }
}

#[test]
fn a_code_block_with_backticks_gets_a_longer_fence() {
    let code = "a ``` b\n````` c\n";
    let html = format!("<pre><code class=\"language-sh\">{code}</code></pre>");
    let written = crate::fuzz::from_html(&html);
    assert!(written.starts_with("``````sh\n"), "{written:?}");
    assert!(written.ends_with("\n``````\n"), "{written:?}");
    assert_eq!(
        meaning(&crate::fuzz::markdown(&written)),
        meaning(&html),
        "{written:?}"
    );

    let span = crate::fuzz::from_html("<p><code>``a`</code></p>");
    assert_eq!(span, "``` ``a` ```\n");
}

/// `fromHtml`'s own linear work, over every catalogued slow from-html input:
/// the bytes its walk reads and copies (`from_html::WORK`), at `n` and `4n`.
/// The bound is five times, as for `toHtml`. The parse's own work is
/// `Core\Html::parse`'s, and its tests bound it.
#[test]
#[cfg_attr(miri, ignore = "documents of thousands of nodes")]
fn from_html_work_grows_linearly_over_every_catalogued_slow_input() {
    const N: usize = 1000;
    let work = |html: &str| {
        let before = from_html::WORK.with(std::cell::Cell::get);
        let _ = crate::fuzz::from_html(html);
        from_html::WORK.with(std::cell::Cell::get) - before
    };
    let short = slow_inputs(N, "from-html");
    let long = slow_inputs(4 * N, "from-html");
    assert!(short.len() >= 10, "the catalogue has its slow entries");
    let report: Vec<String> = short
        .iter()
        .zip(&long)
        .filter_map(|((id, short), (_, long))| {
            let (small, large) = (work(short), work(long));
            (large <= small || large > 5 * small)
                .then(|| format!("{id}: {small} bytes of work at n = {N}, {large} at 4n"))
        })
        .collect();
    assert!(
        report.is_empty(),
        "{} entries grow faster than their input:\n{}",
        report.len(),
        report.join("\n")
    );
}

#[test]
fn from_html_reads_a_node_a_program_already_holds() {
    use nvs_runtime::{Ctx, OutputSink};
    let mut ctx = Ctx::new(OutputSink::Sink);
    let tree = crate::html::parse("<p>a</p><div><script>x()</script><p><em>b</em></p></div>");
    let node = crate::xml::instance_of(tree, &[]);
    let written =
        nvs_runtime::call(nvs_core_markdown_from_html, &mut ctx, &[node]).expect("a node converts");
    assert_eq!(written.as_text(), Some("a\n\n*b*\n"));
    #[expect(
        unsafe_code,
        reason = "the case owns the one reference of each value it built"
    )]
    unsafe {
        written.release();
        node.release();
    }
}

/// `text` cut into pieces of about `size` bytes, each ending on a character.
fn pieces(text: &str, size: usize) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + size).min(text.len());
        while !text.is_char_boundary(end) {
            end += 1;
        }
        out.push(&text[start..end]);
        start = end;
    }
    out
}

/// `rule:core-classes/markdown-from-html` keeps the whole tree, because the
/// tree builder still changes a top-level node after a later one opened. So
/// what streams is the read: the parser takes the document one piece at a
/// time, and the Markdown is the whole tree's for every cut.
#[test]
#[cfg_attr(miri, ignore = "a document of thousands of nodes, parsed once per cut")]
fn from_html_streamed_gives_the_same_markdown_as_the_whole_tree() {
    let block = "<h2 id=\"x\">Größe &amp; <em>Maß</em></h2>\
        <p>a <a href=\"/b\">link</a> and <code>`c`</code>, www.example.com</p>\
        <ul><li>one</li><li><p>two</p><ul><li>[ ] three</li></ul></li></ul>\
        <ul><li>a list right after a list</li></ul>\
        <ol start=\"3\"><li>x</li></ol>\
        text before a table<table>fostered<tr><th>h</th></tr><tr><td>d</td></tr></table>\
        <pre>let a = `b`;\n\n  c</pre><blockquote><p>q</p><hr></blockquote>\
        <dl><dt>t</dt><dd>d</dd></dl><b>open <p>misnested</b> text</p>\
        <script>x()</script>\u{1F600} <!-- c --> tail\n";
    let html = block.repeat(200);
    let whole = from_html::markdown(parse(&html)).expect("the document converts");
    assert!(whole.len() > 50_000, "the document is large");
    for size in [1, 3, 64, 4096] {
        let streamed = from_html::markdown(crate::html::parsed_in_pieces(pieces(&html, size)))
            .expect("the document converts");
        assert_eq!(streamed, whole, "pieces of {size} bytes");
    }
    // Why the tree is kept: `b` arrives after `<table>` opened, and the
    // builder joins it to the text node before the table. A writer that wrote
    // `a` once the table opened would write two paragraphs.
    let fostered = crate::fuzz::from_html("a<table>b");
    assert!(fostered.starts_with("ab\n\n"), "{fostered:?}");
}

/// The renderer's tree is as deep as the document until the rebuild flattens
/// it, so freeing it must not recurse. The thread has the 1 MiB stack of the
/// binary's main thread on Windows, where 4,000 levels overflowed.
#[test]
#[cfg_attr(miri, ignore = "renders 20,000 levels, which Miri takes minutes for")]
fn a_list_nested_far_past_512_renders_on_a_small_stack() {
    let markdown = format!("{}a", "- ".repeat(20_000));
    let out = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(move || rendered(&markdown, "a test").expect("no limit in a test"))
        .expect("a thread")
        .join()
        .expect("no stack overflow");
    assert!(depth(&parse(&out)) <= OPEN_ELEMENTS_CAP);
}

/// Asserts that each of `documents`, rendered, written back by `fromHtml` and
/// rendered again, keeps its meaning, and that `fromHtml` gives the same
/// Markdown from that Markdown's own HTML.
fn keeps_its_meaning(documents: &[&str]) {
    for document in documents {
        let html = crate::fuzz::markdown(document);
        let written = crate::fuzz::from_html(&html);
        let again = crate::fuzz::markdown(&written);
        assert_eq!(
            meaning(&again),
            meaning(&html),
            "{document:?} rendered {html:?}, wrote {written:?}"
        );
        assert_eq!(crate::fuzz::from_html(&again), written, "{document:?}");
    }
}

// The `markdown_from_html` fuzz target found each input below.

// covers: Core\Markdown::fromHtml
#[test]
fn a_tight_item_with_text_after_a_closed_quote_or_list_stays_tight() {
    keeps_its_meaning(&["*\t>\n\t4", "-\t>\n\t8", "- >\r  p", "*\t-\r\t*\r\t\0"]);
}

// covers: Core\Markdown::fromHtml
#[test]
fn a_list_item_that_starts_with_a_rule_keeps_it_after_another_list() {
    keeps_its_meaning(&["*\n- ***", "*\n-\t***"]);
}

// covers: Core\Markdown::fromHtml
#[test]
fn a_break_alone_in_a_link_is_kept() {
    keeps_its_meaning(&["[\\\n]()"]);
}

// covers: Core\Markdown::fromHtml
#[test]
fn a_break_that_ends_link_text_at_the_end_of_a_paragraph_means_nothing() {
    keeps_its_meaning(&[
        "*\u{11}[!+body&\\\\\n\\\n]()\n",
        "[[ul\\\n](`:``)",
        "[[u[[ul\\\n](;,)",
        "a [b\\\n\\\n]()",
    ]);
}

// covers: Core\Markdown::fromHtml
#[test]
fn a_tight_item_with_text_before_a_quote_or_list_stays_tight() {
    keeps_its_meaning(&[
        "-\t\0\0\0\0\r\t>\u{1b}\r\t- ",
        ";`tab>\n\n*\t*>[^\n*\t*\t*\tle<em\n\t>",
        "- a\n  > b\n  - c\n- d",
        "- a\n  > b\n  -\n- d",
        "- a\n  > b\n  2. c\n- d",
        "- a\n  > b\n  >\n  c\n- d",
        "- a\n  > b\n  >\n  | x |\n  |---|\n- d",
        "- a\n  > > b\n  >\n  -\n- d",
        "- a\n  > | x |\n  > |---|\n  >\n  -\n- d",
        "- a\n  > - b\n  > - e\n  >\n  -\n- d",
        "- a\n  >\n  - c\n- d",
        "- a\n  >\n  2.\n- d",
        "- a\n  - b\n  > c\n- d",
        "- a\n  - b\n  # c\n- d",
        "- a\n  - b\n  ***\n- d",
        "- a\n  - b\n  ```\n  x\n  ```\n- d",
        "- a\n  * b\n  -\n- d",
        "- a\n  * b\n  - c\n- d",
    ]);
}

// covers: Core\Markdown::fromHtml
#[test]
fn a_paragraph_whose_only_link_is_dropped_means_nothing() {
    keeps_its_meaning(&["[](:)", "[](//)"]);
}

// covers: Core\Markdown::fromHtml
#[test]
#[cfg_attr(
    miri,
    ignore = "parses and renders dozens of documents, which Miri takes about twenty seconds each for"
)]
fn footnotes_are_written_as_footnotes() {
    keeps_its_meaning(&[
        "[^o]:[^o]",
        "[^#]:[^#]",
        "[^0]:[^0]",
        "[^/]:[^/]",
        "[^:]:[^:]",
        "a[^1]\n\n[^1]: b\n",
        "a[^x](y) b[^x][^z] c[^x]: d [^z]\n\n[^x]: one\n\n[^z]: two\n",
        "a[^1]\n\n[^1]: - a\n    b\n\n    - c\n\n    para\n\n    ```\n    code\n    ```\n",
        "# h[^1]\n\n| a | b[^2] |\n|---|---|\n| 1[^1] | 2 |\n\n[^1]: x[^3]\n[^2]: y\n[^3]: z\n",
        "a[^1]\n\n[^1]:\n",
        "a[^1]\n\n[^1]: > q\n    > r\n",
        "x\n\n***\n\na[^1]\n\n[^1]: n\n",
        "a[^1]\\(x) [^1]\\[y\\] z\n\n[^1]: n\n",
        "[^1]\\: b\n\n[^1]: n\n",
        "b[^x]\n\n[^x]: [^y]\n[^y]: [^x]\n",
    ]);
    // A reference right before a link, and a back link inside the note's
    // paragraph.
    let html = "<p>a<sup><a href=\"#md-fn-1\" id=\"md-fnref-1\">1</a></sup><a href=\"/u\">b</a></p>\
                <hr><ol><li id=\"md-fn-1\"><p>n <a href=\"#md-fnref-1\">\u{21A9}</a></p></li></ol>";
    let written = crate::fuzz::from_html(html);
    assert_eq!(written, "a[^1]<!-- -->[b](/u)\n\n[^1]: n\n");
    assert_eq!(
        crate::fuzz::from_html(&crate::fuzz::markdown(&written)),
        written
    );
    // Notes that no reference with an id shows are named first in a
    // definition the renderer does not write, so they keep their numbers.
    let html = "<p><sup><a href=\"#md-fn-2\">2</a></sup></p><hr><ol>\
                <li id=\"md-fn-1\">a <a href=\"#md-fnref-1\">\u{21A9}</a></li>\
                <li id=\"md-fn-2\">b <a href=\"#md-fnref-2\">\u{21A9}</a></li></ol>";
    let written = crate::fuzz::from_html(html);
    assert_eq!(written, "[^0]: [^1] [^2]\n\n[^2]\n\n[^1]: a\n\n[^2]: b\n");
    assert_eq!(
        crate::fuzz::from_html(&crate::fuzz::markdown(&written)),
        written
    );
    // References with ids whose first appearances are not in the notes'
    // order are written as links, since the renderer would number the notes
    // another way.
    let html = "<p><sup><a href=\"#md-fn-2\" id=\"md-fnref-2\">2</a></sup>\
                <sup><a href=\"#md-fn-1\" id=\"md-fnref-1\">1</a></sup></p><hr><ol>\
                <li id=\"md-fn-1\">a <a href=\"#md-fnref-1\">\u{21A9}</a></li>\
                <li id=\"md-fn-2\">b <a href=\"#md-fnref-2\">\u{21A9}</a></li></ol>";
    let written = crate::fuzz::from_html(html);
    assert!(
        written.starts_with("<sup>[2](#md-fn-2)</sup>"),
        "{written:?}"
    );
    assert_eq!(
        crate::fuzz::from_html(&crate::fuzz::markdown(&written)),
        written
    );
}

/// `fromHtml` over `html`, then over the HTML its Markdown renders to, gives
/// the same Markdown: the `markdown_from_html` fuzz target's first property.
fn assert_written_from_its_own_html(html: &str) -> String {
    let first = crate::fuzz::from_html(html);
    let second = crate::fuzz::from_html(&crate::fuzz::markdown(&first));
    assert_eq!(second, first, "{html:?}");
    first
}

#[test]
fn from_html_keeps_a_carriage_return_in_a_code_block() {
    // The `markdown_from_html` fuzz target found the first.
    assert_written_from_its_own_html("<pre>&#xd");
    assert_written_from_its_own_html("<pre>a&#xd;b&#xd;\nc</pre>");
    assert_written_from_its_own_html(
        "<pre>\n\na&#13;</pre><pre><code class=\"language-x\">&#13;`</code></pre>",
    );
}

#[test]
fn from_html_fences_a_code_span_around_its_runs_of_backticks() {
    // The `markdown_from_html` fuzz target found the first two.
    assert_written_from_its_own_html("<a><code><<a><");
    assert_written_from_its_own_html("<table><code>[<tr><");
    assert_written_from_its_own_html("<p><code>a``b`c</code> <code>```</code></p>");
}

#[test]
fn from_html_writes_raw_html_that_parses_back_to_itself() {
    // The `markdown_from_html` fuzz target found the first.
    assert_written_from_its_own_html("<dl><p><table>");
    assert_written_from_its_own_html(
        "<dl><dd><p>a<table><tr><td>b</td></tr></table></p></dd></dl>",
    );
}

#[test]
fn from_html_strips_controls_around_a_url_as_a_browser_does() {
    // The `markdown_from_html` fuzz target found the first. Written as
    // `%13http:`, the renderer read no valid scheme and dropped the link.
    assert_eq!(
        assert_written_from_its_own_html("<a href=\u{13}http:>"),
        "[](http:)\n"
    );
    assert_eq!(
        assert_written_from_its_own_html("<p><a href=\" /a\u{1}b\u{1f}\">a</a></p>"),
        "[a](/a%01b)\n"
    );
    assert_eq!(
        assert_written_from_its_own_html("<p><a href=\"\u{1}javascript:x\">a</a></p>"),
        "a\n"
    );
}

/// Runs the `markdown_from_html` fuzz target's whole check on each of
/// `inputs`: `fromHtml` over the input read as HTML gives Markdown whose own
/// HTML it writes back unchanged, and the input read as Markdown with no raw
/// HTML keeps its meaning through `fromHtml`.
fn holds_the_fuzz_properties(inputs: &[&str]) {
    for input in inputs {
        assert_written_from_its_own_html(input);
        if !crate::fuzz::markdown_has_raw_html(input) {
            keeps_its_meaning(&[input]);
        }
    }
}

// The `markdown_from_html` fuzz target found the first input of each test
// below, or the first few.

// covers: Core\Markdown::fromHtml
#[test]
fn a_task_marker_before_a_break_stays_a_checkbox() {
    holds_the_fuzz_properties(&[
        "- [ ]  \n\0",
        "- [ ]\n\\\n]",
        "- [x]\n\\\n\\\nb",
        "<ul><li><input type=\"checkbox\" checked><br><br></li></ul>",
    ]);
}

// covers: Core\Markdown::fromHtml
#[test]
fn a_tight_item_with_text_before_a_table_stays_tight() {
    holds_the_fuzz_properties(&[
        "- l\rl\r  :-",
        "- \\\rd\r  :-",
        "- \\\ny\r  :-",
        "- :\ri\r  :-",
        "-\t\0\n\u{2}\n    -:",
        "- a\\\n  b\n  | x | y |\n  | --- | --- |\n  | 1 | 2 |\n- c",
    ]);
}

// covers: Core\Markdown::fromHtml
#[test]
fn a_list_whose_items_write_nothing_is_written_tight() {
    holds_the_fuzz_properties(&["<ul><p><li>", "<ul><li><p></p></li><li><p> </p></li></ul>"]);
}

// covers: Core\Markdown::fromHtml
#[test]
#[cfg_attr(
    miri,
    ignore = "parses and renders dozens of documents, which Miri takes about twenty seconds each for"
)]
fn footnotes_keep_their_numbers_and_ids_in_any_section() {
    holds_the_fuzz_properties(&[
        "[^\0]:[^\0]:[^\0]",
        "[^;]:[^;]:[^;]",
        "[^\0]:[^\u{1c}]\n[^\u{1c}]:",
        "[^,,,]:[^,,,]\n\n[^,,,]",
        "[^a]: [^b] [^c]\n\n[^b]: x\n\n[^c]: y",
        "a[^1]\n\n[^0]: [^2]\n\n[^1]: x\n\n[^2]: y",
        "[^1]: [^1]\n\n[^0]: [^2]\n\nb [^1] [^2]\n\n[^2]: z",
        "[^0]: [^1]\n\nb [^1]\n\n[^1]: z [^1]",
        "a[^1] [^2]\n\n[^1]: b[^2]\n\n[^2]: c",
        "[^1]: [^2]\n\n[^2]: [^1]\n\n[^1]",
        "[^h]: [^a]\n\nb[^b]\n\n[^g]: [^c]\n\n[^a]: 1\n\n[^b]: 2\n\n[^c]: 3",
    ]);
    let html =
        crate::fuzz::markdown("[^h]: [^a]\n\nb[^b]\n\n[^g]: [^c]\n\n[^a]: 1\n\n[^b]: 2\n\n[^c]: 3");
    assert_eq!(
        crate::fuzz::from_html(&html),
        "[^0]: [^1]\n\nb[^2]\n\n[^0-1]: [^3]\n\n[^1]: 1\n\n[^2]: 2\n\n[^3]: 3\n"
    );
    // A note none of whose references is shown is named in `[^0]:`, a
    // definition no reference names, which the renderer does not write.
    let html = crate::fuzz::markdown("[^\0]:[^\0]:[^\0]");
    assert_eq!(crate::fuzz::from_html(&html), "[^0]: [^1]\n\n[^1]:\n");
    // A note whose own reference to itself has the id goes before the body.
    let html = crate::fuzz::markdown("[^,,,]:[^,,,]\n\n[^,,,]");
    assert_eq!(crate::fuzz::from_html(&html), "[^1]: [^1]\n\n[^1]\n");
}
