//! The one reordering the formatter performs, as
//! `rule:tooling/fmt-sorts-the-use-block` states it.
//!
//! Each case is one file and the canonical text it formats to, so what a
//! failure prints is the whole disagreement rather than a property that went
//! false somewhere. An input is wrong about the rule under test and about
//! nothing else — every other byte of it is what the printer answers today, so
//! a case fails for its own reason and an unlanded rule cannot mask it.

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `source`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(source: &str) -> String {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs".to_string(), source);
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

#[test]
fn consecutive_use_declarations_are_sorted_by_full_path() {
    let mangled = "\
<?nvs
use Core\\Str;
use App\\Queue\\Worker;
use Core\\Arr;
use app\\Queue;

class Job
{
}
";
    let canonical = "\
<?nvs
use App\\Queue\\Worker;
use Core\\Arr;
use Core\\Str;
use app\\Queue;

class Job
{
}
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_comment_moves_with_the_import_it_was_written_above_or_beside() {
    let source = "\
<?nvs
use Core\\Str;
// Runs the jobs.
/* One per queue. */
use App\\Worker;
use App\\Queue; // the queue itself
";
    let sorted = "\
<?nvs
use App\\Queue; // the queue itself
// Runs the jobs.
/* One per queue. */
use App\\Worker;
use Core\\Str;
";
    assert_eq!(formatted(source), sorted);
}

#[test]
fn a_comment_a_blank_line_away_from_the_next_import_holds_the_block_apart() {
    let source = "\
<?nvs
use Core\\Str;
// The two below are the queue's.

use App\\Worker;
use App\\Queue;
";
    let sorted = "\
<?nvs
use Core\\Str;
// The two below are the queue's.

use App\\Queue;
use App\\Worker;
";
    assert_eq!(formatted(source), sorted);
}
