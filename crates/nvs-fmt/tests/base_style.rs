//! PER's base style, as `rule:tooling/fmt-base-style-is-per` states it.
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
fn indentation_is_four_spaces_per_block_depth() {
    let mangled = "\
<?nvs
class Tag
{
      public string $name;

  public function rename(string $name): void
    {
$this->name = $name;
        if ($name == '') {
  $this->name = 'untitled';
          }
  }
}

  var $tag = new Tag('red');
$tag->rename('');
";
    let canonical = "\
<?nvs
class Tag
{
    public string $name;

    public function rename(string $name): void
    {
        $this->name = $name;
        if ($name == '') {
            $this->name = 'untitled';
        }
    }
}

var $tag = new Tag('red');
$tag->rename('');
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn modifiers_are_written_in_the_canonical_order() {
    let mangled = "\
<?nvs
abstract class Cache
{
    static private int $hits = 0;
    readonly private string $name;

    public abstract function warm(): void;

    final public function reset(): void
    {
    }
}
";
    let canonical = "\
<?nvs
abstract class Cache
{
    private static int $hits = 0;
    private readonly string $name;

    abstract public function warm(): void;

    final public function reset(): void
    {
    }
}
";
    assert_eq!(formatted(mangled), canonical);
}

/// `covariant` is parsed only between the modifiers and the type, so the
/// modifiers move around it and it stays last.
#[test]
fn covariant_is_written_after_every_other_modifier() {
    let mangled = "\
<?nvs
class Kennel extends Shelter
{
    lateinit public covariant Dog $pet;

    public function constructor(readonly public covariant Dog $first)
    {
    }

    public function feed(inout covariant Dog $a, covariant Dog ...$all): void
    {
    }
}
";
    let canonical = "\
<?nvs
class Kennel extends Shelter
{
    public lateinit covariant Dog $pet;

    public function constructor(public readonly covariant Dog $first)
    {
    }

    public function feed(inout covariant Dog $a, covariant Dog ...$all): void
    {
    }
}
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_declaration_brace_is_allman_and_a_control_brace_is_k_and_r() {
    let mangled = "\
<?nvs
class Queue {
    public function drain(int $limit): void {
        for ($i = 0; $i < $limit; $i++)
        {
            if ($i == 0)
            {
                continue;
            }
            elseif ($i == 1)
            {
                break;
            }
            else
            {
                echo $i;
            }
        }
        try
        {
            echo 'done';
        }
        catch (Error $e)
        {
            echo 'failed';
        }
    }
}
";
    let canonical = "\
<?nvs
class Queue
{
    public function drain(int $limit): void
    {
        for ($i = 0; $i < $limit; $i++) {
            if ($i == 0) {
                continue;
            } elseif ($i == 1) {
                break;
            } else {
                echo $i;
            }
        }
        try {
            echo 'done';
        } catch (Error $e) {
            echo 'failed';
        }
    }
}
";
    assert_eq!(formatted(mangled), canonical);
}

/// A `for` header's clauses and a one-line anonymous function are not two
/// statements, and an empty body keeps its braces together.
#[test]
fn two_statements_on_one_line_are_split_onto_two() {
    let mangled = "\
<?nvs
class Counter
{
    public int $a = 0; public int $b = 0;

    public function reset(): void
    {
    }
}

var $c = new Counter(); $c->a = 1;
for ($i = 0; $i < 2; $i++) {
    $c->a = $c->a + $i; $c->b = $c->b + 1;
}
var $twice = fn(int $n): int => { int $m = $n * 2; return $m; };
";
    let canonical = "\
<?nvs
class Counter
{
    public int $a = 0;
    public int $b = 0;

    public function reset(): void
    {
    }
}

var $c = new Counter();
$c->a = 1;
for ($i = 0; $i < 2; $i++) {
    $c->a = $c->a + $i;
    $c->b = $c->b + 1;
}
var $twice = fn(int $n): int => { int $m = $n * 2; return $m; };
";
    assert_eq!(formatted(mangled), canonical);
}

/// A body written on one line is opened out, and what it contains is indented
/// from the line its declaration now starts on.
#[test]
fn a_closing_brace_after_a_statement_starts_its_own_line() {
    let mangled = "\
<?nvs
class Page
{ public function render(): string
{ return 'home'; } }

if ($ok) { echo 'yes'; }
try { risky(); } catch (Error $e) { echo 'no';}
";
    let canonical = "\
<?nvs
class Page
{
    public function render(): string
    {
        return 'home';
    }
}

if ($ok) {
    echo 'yes';
}
try {
    risky();
} catch (Error $e) {
    echo 'no';
}
";
    assert_eq!(formatted(mangled), canonical);
}

/// A comment between the two words, and the words inside a string, are the
/// author's.
#[test]
fn else_if_is_written_as_one_word() {
    let mangled = "\
<?nvs
if ($a) {
    echo 'else if';
} else if ($b) {
    echo 'b';
} else /* two words */ if ($c) {
    echo 'c';
} else
if ($d) {
    echo 'd';
}
";
    let canonical = "\
<?nvs
if ($a) {
    echo 'else if';
} elseif ($b) {
    echo 'b';
} else /* two words */ if ($c) {
    echo 'c';
} elseif ($d) {
    echo 'd';
}
";
    assert_eq!(formatted(mangled), canonical);
}

/// Three blank lines in a row are one, between two statements and between two
/// top-level declarations, and a single blank line the author left stands
/// (`rule:tooling/fmt-at-most-one-blank-line`).
#[test]
fn a_run_of_blank_lines_is_one_blank_line() {
    let mangled = "\
<?nvs
int $a = 1;



int $b = 2;

int $c = 3;


class Cart
{
    public int $count = 0;
}
";
    let canonical = "\
<?nvs
int $a = 1;

int $b = 2;

int $c = 3;

class Cart
{
    public int $count = 0;
}
";
    assert_eq!(formatted(mangled), canonical);
}

/// A blank line right after a body's `{` or a list's `(`, and right before its
/// `}` or `)`, is deleted, and a blank line between two statements inside the
/// body stands.
#[test]
fn a_body_has_no_blank_line_at_its_inside_edge() {
    let mangled = "\
<?nvs
class Cart
{

    public function add(int $n): int
    {

        int $total = $n;

        return sum(

            $total,
            1,

        );

    }

}
";
    let canonical = "\
<?nvs
class Cart
{
    public function add(int $n): int
    {
        int $total = $n;

        return sum(
            $total,
            1,
        );
    }
}
";
    assert_eq!(formatted(mangled), canonical);
}

/// The blank lines inside a heredoc's body and an html template's body are
/// text the program prints, so they are copied as they are.
#[test]
fn a_heredoc_and_a_template_keep_their_blank_lines() {
    let source = "\
<?nvs
string $text = <<<'TEXT'
one



two
TEXT;
echo html`<p>



</p>`;
";
    assert_eq!(formatted(source), source);
}
