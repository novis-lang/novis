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

/// `??+=`, `??-=` and `??.=` (`rule:expressions/defaulting-assignment`) are
/// each one token, so the printer keeps each whole and leaves the one space
/// either side as written, as it does around every other assignment operator,
/// and the file is formatted rather than refused.
#[test]
fn a_defaulting_assignment_operator_is_spaced_as_an_assignment() {
    let canonical = "\
<?nvs
class Shop
{
    public function tally(array<int> $counts, ?string $log): void
    {
        $counts['a'] ??+= 1;
        $counts['b'] ??-= 2;
        $log ??.= 'x';
        $counts['c'] += 1;
    }
}
";
    assert_eq!(formatted(canonical), canonical);
}

#[test]
fn a_qualifier_sits_one_space_before_its_type() {
    // A property, a parameter, a return type and a local, each written with a
    // run of spaces where one belongs, in front of a scalar and in front of a
    // second qualifier. The `tainted` written as a member name and the one
    // written inside a string are not qualifiers at all, so the run in front of
    // `is` and the `?` inside the literal are the author's.
    let mangled = "\
<?nvs
class Inbox
{
    public tainted    string $subject = 'none';
    public ?Inbox $tainted = null;

    public function stamp(secret  tainted string $token, ?tainted string $note): tainted   bytes
    {
        tainted  string $line = $note ?? $this->subject;
        if ($this->tainted   is Inbox) {
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
        if ($this->tainted   is Inbox) {
            echo 'is it tainted?really', \"\\n\";
        }
        return $line as bytes;
    }
}
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_class_reference_after_is_is_formatted_as_a_type_is() {
    // `$x is $cls` is the one right-hand side of `is` that is a value rather
    // than a type (`rule:types/type-test`'s value arm), and the printer's whole
    // job there is to put it where a written type goes. The two guards are
    // mangled identically — an indent the printer owns — so a value arm laid
    // out by any other rule comes back unlike the type form beside it.
    let mangled = "\
<?nvs
class Inbox
{
    public function look(mixed $m, class<Inbox> $cls): bool
    {
          if ($m is Inbox) {
            return true;
        }
          if ($m is $cls) {
            return true;
        }
        return false;
    }
}
";
    let canonical = "\
<?nvs
class Inbox
{
    public function look(mixed $m, class<Inbox> $cls): bool
    {
        if ($m is Inbox) {
            return true;
        }
        if ($m is $cls) {
            return true;
        }
        return false;
    }
}
";
    let printed = formatted(mangled);
    assert_eq!(printed, canonical);

    let guards: Vec<&str> = printed
        .lines()
        .filter(|line| line.contains(" is "))
        .collect();
    assert_eq!(
        guards.len(),
        2,
        "the case writes the two spellings of the one type test and nothing else"
    );
    assert_eq!(
        guards[0].replace("Inbox", "$cls"),
        guards[1],
        "the class reference after `is` is not laid out where the written type is"
    );
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
fn a_class_scoped_type_alias_has_one_layout() {
    // A `type` member is a declaration with no body, so its whole layout is the
    // depth of the body it sits in: four spaces per enclosing body, no tabs
    // (`rule:tooling/fmt-base-style-is-per`), in a class, an interface and an
    // enum alike. Each owner here writes one at a depth nobody would choose —
    // too deep, at the left margin, half a level, a tab — and every other byte
    // is what the printer answers today. The runs inside the type expression
    // are among them: a type is a node to nobody in
    // `crates/nvs-syntax/src/walk.rs`, so the `{` of a shape written in type
    // position is outside the one-space rule the object literal below is
    // inside, which is the crate's own known gap `nvs-fmt/comma-separated-list-whose-closing-delimiter` and is the same at file
    // scope. The blank line each author left is theirs, here as everywhere
    // (known gap 1).
    let mangled = "\
<?nvs
class Order
{
        type Meta = {total: decimal, note?: string};
type Tag = string|int;

    public function stamp(Meta $meta): Tag
    {
        return $meta->note ?? 'none';
    }
}

interface Shipper
{
  type Label = {code: string};
}

enum Status
{
\ttype Pair = array<Status>;

    Open,
    Shipped,
}

class Report
{
    public static function total(Order::Meta $meta): decimal
    {
        return $meta->total;
    }
}
echo Report::total({ total: 2.5 }), \"\\n\";
";
    let canonical = "\
<?nvs
class Order
{
    type Meta = {total: decimal, note?: string};
    type Tag = string|int;

    public function stamp(Meta $meta): Tag
    {
        return $meta->note ?? 'none';
    }
}

interface Shipper
{
    type Label = {code: string};
}

enum Status
{
    type Pair = array<Status>;

    Open,
    Shipped,
}

class Report
{
    public static function total(Order::Meta $meta): decimal
    {
        return $meta->total;
    }
}
echo Report::total({ total: 2.5 }), \"\\n\";
";
    assert_eq!(formatted(mangled), canonical);
    // And the layout is one layout: what comes out goes back in unchanged
    // (`rule:tooling/fmt-is-idempotent`), so neither the member's own line nor
    // the bare `Meta` and `Order::Meta` that reach it move on a second run.
    assert_eq!(formatted(canonical), canonical);
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
