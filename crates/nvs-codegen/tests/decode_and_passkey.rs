//! A typed decode bounds its issue list, and a passkey challenge is taken once
//! (`rule:core-classes/derive-reports-every-field`).
//!
//! - A list of many bad elements ends in 64 issues and one closing issue at
//!   the root's path, and the message says "more than 64". The decode stays
//!   inside the default memory limit.
//! - A list with 64 bad elements lists all 64 and no closing issue.
//!
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;

/// A program that decodes a list of `count` strings into an `array<int>`
/// property, and prints the error's message, its issue count and its last
/// issue.
fn bad_list(count: usize) -> String {
    format!(
        r#"<?nvs
#[Core\Json\Derive]
class Box
{{
    public array<int> $xs;

    public function constructor(array<int> $xs)
    {{
        $this->xs = $xs;
    }}
}}

string $text = '{{"xs":[' . Core\Str::repeat('"x",', {before}) . '"x"]}}';
try {{
    Core\Json::decodeAs<Box>($text);
    echo "decoded\n";
}} catch (ParseError $error) {{
    echo $error->message, "\n";
    echo Core\Arr::count($error->issues), "\n";
    string $last = '';
    foreach ($error->issues as {{ path: string, message: string }} $issue) {{
        $last = $issue->path . ': ' . $issue->message;
    }}
    echo $last, "\n";
}}
"#,
        before = count - 1
    )
}

#[test]
fn a_list_of_many_bad_elements_ends_in_a_bounded_issue_list() {
    let output = output_of(&bad_list(1_000_000));
    let lines: Vec<&str> = output.lines().collect();
    assert!(
        lines[0].contains("more than 64 value(s) of `Box` did not match"),
        "{output}"
    );
    assert_eq!(lines[1], "65", "{output}");
    assert_eq!(
        lines[2], ": More values did not match. Only the first 64 problems are listed.",
        "{output}"
    );
}

#[test]
fn a_list_of_exactly_the_bound_lists_every_issue() {
    let output = output_of(&bad_list(64));
    let lines: Vec<&str> = output.lines().collect();
    assert!(
        lines[0].contains("64 value(s) of `Box` did not match"),
        "{output}"
    );
    assert_eq!(lines[1], "64", "{output}");
    assert!(lines[2].starts_with("xs.63: expected"), "{output}");
}
