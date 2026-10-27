//! The run between two tokens on one line, as
//! `rule:tooling/fmt-one-run-between-two-tokens` states it.
//!
//! Each case is one file and the canonical text it formats to, one row of the
//! table at a time. An input is wrong about its row and about nothing else:
//! every other byte of it is what the printer answers today, so a case fails
//! for its own reason and another row cannot mask it.

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `source`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(source: &str) -> String {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs".to_string(), source);
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

/// The class every case that needs a method writes its lines into.
fn in_a_method(lines: &str) -> String {
    format!(
        "<?nvs\n\nclass Cart\n{{\n    public function total(int $a, int $b): int\n    {{\n{lines}    }}\n}}\n"
    )
}

#[test]
fn a_binary_operator_has_one_space_on_each_side() {
    let mangled = "\
<?nvs
int $a = 1;
int $b = 2;
int $sum = $a+$b*2;
bool $both = $a>0&&$b<3;
string $text = 'n: '.$sum;
int $kept  =  $a ?? $b;
$sum+=1;
$sum = $a-$b;
";
    let canonical = "\
<?nvs
int $a = 1;
int $b = 2;
int $sum = $a + $b * 2;
bool $both = $a > 0 && $b < 3;
string $text = 'n: ' . $sum;
int $kept = $a ?? $b;
$sum += 1;
$sum = $a - $b;
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_fat_arrow_has_one_space_on_each_side() {
    let mangled = "\
<?nvs
array<int> $prices = ['tea'=>3, 'cake'  =>  4];
array<int> $doubled = Core\\Arr::map([1, 2], fn(int $x)=>$x * 2);
";
    let canonical = "\
<?nvs
array<int> $prices = ['tea' => 3, 'cake' => 4];
array<int> $doubled = Core\\Arr::map([1, 2], fn(int $x) => $x * 2);
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_ternary_has_one_space_around_each_mark_and_none_inside_its_short_form() {
    let mangled = "\
<?nvs
int $a = 1;
int $b = $a>0?$a:2;
int $c = $a ? : $b;
";
    let canonical = "\
<?nvs
int $a = 1;
int $b = $a > 0 ? $a : 2;
int $c = $a ?: $b;
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_comma_has_nothing_before_it_and_one_space_after_it() {
    let mangled = "\
<?nvs
array<int> $list = [1 ,2,  3];
echo $list[0] , \"\\n\";
";
    let canonical = "\
<?nvs
array<int> $list = [1, 2, 3];
echo $list[0], \"\\n\";
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_semicolon_has_nothing_before_it_and_one_space_after_it_in_a_for_header() {
    let mangled = "\
<?nvs
int $total = 0 ;
for (int $i = 0;$i < 3;  $i++) {
    $total += $i;
}
";
    let canonical = "\
<?nvs
int $total = 0;
for (int $i = 0; $i < 3; $i++) {
    $total += $i;
}
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_bracket_has_nothing_inside_it() {
    let mangled = "\
<?nvs
array<int> $list = [ 1, 2 ];
int $first = ( $list[ 0 ] );
";
    let canonical = "\
<?nvs
array<int> $list = [1, 2];
int $first = ($list[0]);
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_callee_has_nothing_before_its_parenthesis() {
    let mangled = "\
<?nvs
array<int> $list = [1, 2];
int $size = Core\\Arr::count ($list);
bool $set = isset ($list[0]);
array<int> $same = Core\\Arr::map($list, fn ($x) => $x);
";
    let canonical = "\
<?nvs
array<int> $list = [1, 2];
int $size = Core\\Arr::count($list);
bool $set = isset($list[0]);
array<int> $same = Core\\Arr::map($list, fn($x) => $x);
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_declared_name_has_nothing_before_its_parenthesis() {
    let mangled = in_a_method("        return $a;\n").replace("total(", "total (");
    let canonical = in_a_method("        return $a;\n");
    assert_eq!(formatted(&mangled), canonical);
}

#[test]
fn an_index_has_nothing_before_its_bracket() {
    let mangled = "\
<?nvs
array<int> $list = [1, 2];
int $first = $list [0];
";
    let canonical = "\
<?nvs
array<int> $list = [1, 2];
int $first = $list[0];
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_control_keyword_has_one_space_before_its_parenthesis() {
    let mangled = "\
<?nvs
int $a = 1;
if($a > 0) {
    echo 'yes';
}
while  ($a < 3) {
    $a++;
}
";
    let canonical = "\
<?nvs
int $a = 1;
if ($a > 0) {
    echo 'yes';
}
while ($a < 3) {
    $a++;
}
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_return_type_colon_has_nothing_before_it_and_one_space_after_it() {
    let mangled = in_a_method("        return $a;\n").replace("): int", ") :int");
    let canonical = in_a_method("        return $a;\n");
    assert_eq!(formatted(&mangled), canonical);
}

#[test]
fn an_object_field_and_an_enum_backing_type_space_their_colon() {
    let mangled = "\
<?nvs

enum Priority : int
{
    High = 30,
    Low = 10,
}

array<int> $odd = Core\\Arr::range(1, 10, { step :2 });
";
    let canonical = "\
<?nvs

enum Priority: int
{
    High = 30,
    Low = 10,
}

array<int> $odd = Core\\Arr::range(1, 10, { step: 2 });
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_named_argument_has_nothing_before_its_colon_and_one_space_after_it() {
    let mangled = "\
<?nvs

class Greet
{
    #[Doc(text :'a')]
    public string $name;
}
";
    let canonical = "\
<?nvs

class Greet
{
    #[Doc(text: 'a')]
    public string $name;
}
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_case_label_has_nothing_before_its_colon() {
    let mangled = "\
<?nvs
string $level = 'debug';
switch ($level) {
    case 'debug' :
        echo 'd';
        break;
    default :
        echo 'other';
}
";
    let canonical = "\
<?nvs
string $level = 'debug';
switch ($level) {
    case 'debug':
        echo 'd';
        break;
    default:
        echo 'other';
}
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn an_arrow_and_a_double_colon_have_nothing_on_either_side() {
    let mangled = "\
<?nvs
string $text = Core\\Str :: upper('a');
int $size = Core\\Str::length($text) ;
echo $text ?-> length;
";
    let canonical = "\
<?nvs
string $text = Core\\Str::upper('a');
int $size = Core\\Str::length($text);
echo $text?->length;
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn an_attribute_has_nothing_inside_its_brackets() {
    let mangled = "\
<?nvs

#[ Core\\Json\\Derive ]
class Search
{
    public string $term;
}
";
    let canonical = "\
<?nvs

#[Core\\Json\\Derive]
class Search
{
    public string $term;
}
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_keyword_has_one_space_before_what_follows_it() {
    let mangled = in_a_method("        return  $a;\n");
    let canonical = in_a_method("        return $a;\n");
    assert_eq!(formatted(&mangled), canonical);
}

#[test]
fn two_words_have_one_space_between_them() {
    let mangled = in_a_method("        return $a;\n").replace("int $b", "int  $b");
    let canonical = in_a_method("        return $a;\n");
    assert_eq!(formatted(&mangled), canonical);
}

#[test]
fn a_comment_after_code_has_one_space_before_it() {
    let mangled = "\
<?nvs
int $a = 1;// one
int $b = 2;    // two
";
    let canonical = "\
<?nvs
int $a = 1; // one
int $b = 2; // two
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_written_type_has_nothing_around_its_punctuation() {
    let signature = "total(?int $a, int|string $b, Pair<string, int> $c, array<array<int> > $d)";
    let canonical = in_a_method("        return 1;\n").replace("total(int $a, int $b)", signature);
    let mangled = canonical
        .replace("?int", "? int")
        .replace("int|string", "int | string")
        .replace("Pair<string, int>", "Pair < string, int >");
    assert_eq!(formatted(&mangled), canonical);
}

#[test]
fn a_signature_type_has_nothing_around_its_punctuation() {
    let mangled = in_a_method("        return $a;\n").replace("): int", "): ? int");
    let canonical = in_a_method("        return $a;\n").replace("): int", "): ?int");
    assert_eq!(formatted(&mangled), canonical);
}

#[test]
fn a_colon_inside_a_type_keeps_its_space_before_a_nullable_type() {
    let mangled = "\
<?nvs

type Size = { width:?int, height: ?int, unit:string, scale: callable(int):?float };
";
    let canonical = "\
<?nvs

type Size = { width: ?int, height: ?int, unit: string, scale: callable(int): ?float };
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_unary_operator_has_nothing_before_its_operand() {
    let mangled = "\
<?nvs
int $x = 1;
bool $ok = true;
int $neg = - $x;
bool $not = ! $ok;
$x ++;
-- $x;
int $kept = $x - - $x;
";
    let canonical = "\
<?nvs
int $x = 1;
bool $ok = true;
int $neg = -$x;
bool $not = !$ok;
$x++;
--$x;
int $kept = $x - -$x;
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_run_beside_a_block_comment_a_tag_or_an_unnamed_pair_is_the_authors() {
    let canonical = "\
<?nvs
int $a = /* one */  1;
echo  -$a;
?>
<p><?=  $a  ?></p>
";
    assert_eq!(formatted(canonical), canonical);
}

/// The variants of `pub enum TokenKind`, read from the lexer's own source so
/// that a kind added there is a kind this test sees.
fn token_kinds() -> Vec<&'static str> {
    let source = include_str!("../../nvs-syntax/src/token.rs");
    let start = source
        .find("pub enum TokenKind {")
        .expect("token.rs declares `pub enum TokenKind`");
    let body = &source[start..];
    let end = body.find("\n}").expect("`TokenKind` has a closing brace");
    body[..end]
        .lines()
        .skip(1)
        .filter_map(|line| {
            let rest = line.strip_prefix("    ")?;
            let name: &str = rest
                .split(|c: char| !c.is_ascii_alphanumeric())
                .next()
                .unwrap_or("");
            name.starts_with(|c: char| c.is_ascii_uppercase())
                .then_some(name)
        })
        .collect()
}

/// `rule:tooling/fmt-one-run-between-two-tokens`: a token kind is either in the
/// table or on its list of kinds whose runs are the author's, so a new kind is
/// never spaced by a guess.
#[test]
fn every_token_kind_has_a_row_or_is_the_authors() {
    let table = include_str!("../src/gap.rs");
    let kinds = token_kinds();
    assert!(
        kinds.len() > 80,
        "only {} `TokenKind` variant(s) were read from token.rs, so this test reads it wrong",
        kinds.len()
    );
    let missing: Vec<&str> = kinds
        .into_iter()
        .filter(|kind| {
            let spelled = format!("TokenKind::{kind}");
            !table.match_indices(&spelled).any(|(at, _)| {
                !table[at + spelled.len()..].starts_with(|c: char| c.is_ascii_alphanumeric())
            })
        })
        .collect();
    assert!(
        missing.is_empty(),
        "`crates/nvs-fmt/src/gap.rs` names no row for these token kinds and does not list them \
         on `AUTHORS`: {missing:?}"
    );
}

#[test]
fn text_a_program_prints_and_a_unary_operator_keep_their_runs() {
    let canonical = "\
<?nvs
int $a = 1;
string $text = \"a+b  {$a}\";
int $b = $a - -$a;
echo $text, $b;
";
    assert_eq!(formatted(canonical), canonical);
}
