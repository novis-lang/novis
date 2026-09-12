//! The constructs PER never saw, as `rule:tooling/fmt-novis-constructs` lays
//! them out.
//!
//! Each case is one mangled file and the canonical text it formats to, so what
//! a failure prints is the whole disagreement rather than a property that went
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
fn a_qualifier_sits_one_space_before_its_type() {
    // A property, a parameter, a return type and a local, each written with a
    // run of spaces where one belongs, in front of a scalar and in front of a
    // second qualifier. The `tainted` written as a member name and the one
    // written inside a string are not qualifiers at all, so the run in front of
    // `instanceof` and the `?` inside the literal are the author's.
    let mangled = "\
<?nvs
class Inbox
{
    public tainted    string $subject = 'none';
    public ?Inbox $tainted = null;

    public function stamp(secret  tainted string $token, ?tainted string $note): tainted   bytes
    {
        tainted  string $line = $note ?? $this->subject;
        if ($this->tainted   instanceof Inbox) {
            echo 'is it tainted?really', \"\\n\";
        }
        return $line as bytes;
    }
}
";
    let canonical = "\
<?nvs
class Inbox
{
    public tainted string $subject = 'none';
    public ?Inbox $tainted = null;

    public function stamp(secret tainted string $token, ?tainted string $note): tainted bytes
    {
        tainted string $line = $note ?? $this->subject;
        if ($this->tainted   instanceof Inbox) {
            echo 'is it tainted?really', \"\\n\";
        }
        return $line as bytes;
    }
}
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_fn_closure_body_brace_stays_on_its_signature_line() {
    // A closure is an expression, so the brace that opens its body never
    // leaves the line its parameter list, return type and `=>` were written
    // on. A body already written there is a fixed point, and a multi-line one
    // keeps its closing brace on a line of its own.
    let mangled = "\
<?nvs
var $double = fn(int $x): int =>
{
    return $x * 2;
};
var $shout = fn(string $s): string => { return Core\\Str::upper($s); };
echo $double(21), $shout('novis'), \"\\n\";
";
    let canonical = "\
<?nvs
var $double = fn(int $x): int => {
    return $x * 2;
};
var $shout = fn(string $s): string => { return Core\\Str::upper($s); };
echo $double(21), $shout('novis'), \"\\n\";
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_match_arm_list_written_across_lines_is_one_arm_per_line() {
    // The tall list has two arms crammed onto the line its `{` opened, one at
    // the wrong depth, and a `default` arm no condition node covers; the blank
    // line an author left between two arms is theirs. The wide list was written
    // on one line, so it stays on one — which is the choice
    // `rule:tooling/fmt-never-reflows` leaves to its author.
    let mangled = "\
<?nvs
class Grade
{
    public static function letter(int $score): string
    {
        return match (true) { $score >= 90 => 'A', $score >= 80 => 'B',
                  $score >= 70 => 'C',

    default => 'F',
};
    }
}
echo Grade::letter(84), match (1) { 1 => 'one', default => 'many' }, \"\\n\";
";
    let canonical = "\
<?nvs
class Grade
{
    public static function letter(int $score): string
    {
        return match (true) {
            $score >= 90 => 'A',
            $score >= 80 => 'B',
            $score >= 70 => 'C',

            default => 'F',
        };
    }
}
echo Grade::letter(84), match (1) { 1 => 'one', default => 'many' }, \"\\n\";
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn an_object_literal_on_one_line_has_one_space_inside_each_brace() {
    // One literal with no space inside either brace and one with too much, and
    // a literal its author wrote across lines, whose fields are one per line
    // and not this rule's at all.
    let mangled = "\
<?nvs
var $point = {x: 1, y: 2};
var $spaced = {   name: 'novis', kind: 'lang'   };
var $wrapped = {
    first: 'a',
    second: 'b',
};
";
    let canonical = "\
<?nvs
var $point = { x: 1, y: 2 };
var $spaced = { name: 'novis', kind: 'lang' };
var $wrapped = {
    first: 'a',
    second: 'b',
};
";
    assert_eq!(formatted(mangled), canonical);
}
