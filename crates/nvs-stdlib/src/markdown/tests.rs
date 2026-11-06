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
            let text = field(bug, "input") + &field(bug, "repeat").repeat(n) + &field(bug, "suffix");
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
