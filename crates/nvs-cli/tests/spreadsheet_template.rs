//! `Novis\Spreadsheet\Template` and `Row`, run through the built binary: a template opened from bytes
//! or from an open file fills its placeholders and repeats its item row, `set` fills one cell by
//! address, and a `Row` carries the number and values `Reader::rows` read, as
//! `rule:core-classes/spreadsheet-template-fill` places them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("spreadsheet-template")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `body` after the `use` lines in the case's own directory, with `toml` as its `nvs.toml`
/// when given, and returns its stdout, failing the test on a non-zero exit.
fn run(case: &str, toml: Option<&str>, body: &str) -> String {
    let dir = scratch(case);
    let program = format!(
        "<?nvs\nuse Novis\\Spreadsheet\\Formula;\nuse Novis\\Spreadsheet\\Reader;\nuse Novis\\Spreadsheet\\Row;\nuse Novis\\Spreadsheet\\Template;\nuse Novis\\Spreadsheet\\Workbook;\n{body}"
    );
    fs::write(dir.join("main.nvs"), program).unwrap();
    if let Some(toml) = toml {
        fs::write(dir.join("nvs.toml"), toml).unwrap();
    }
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("run")
        .arg("main.nvs")
        .current_dir(&dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(
        out.status.success(),
        "the program runs: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("the output is UTF-8")
}

/// The template every case fills: a heading row, an item row and a sum below it.
const DESIGN: &str = r#"
var $design = Workbook::createInMemory();
$design->sheet("Invoice")->addAll([
    ["Invoice {{number}}", "{{customer.name}}"],
    ["{{items.name}}", "{{items.qty}}", Formula::of("B2*2")],
    ["Sum", Formula::of("SUM(B2:B2)")],
]);
bytes $template = $design->finish()->bytes ?? Core\Bytes::fill(0, 0);
"#;

/// Prints each row of `$file`'s sheet `Invoice` as its number and its values, formulas as text.
const SHOW: &str = r#"
final class Show {
    public static function rows(bytes $file): void {
        foreach (Reader::open($file)->rows("Invoice", {formulas: true}) as Row $row) {
            echo $row->number;
            foreach ($row->values as tainted string $key => mixed $value) {
                echo " ", $key, "=", $value is Formula ? "=" . $value->text : Core\Json::encode($value);
            }
            echo "\n";
        }
    }
}
"#;

/// `fill` writes the item row once per item, moves its formula to each copy and grows the sum,
/// and each `Row` it reads back has its row number and its values by column letter.
// covers: Novis\Spreadsheet\Template::open, Novis\Spreadsheet\Template::fill, Novis\Spreadsheet\Row::constructor
#[test]
fn fill_repeats_the_item_row_and_grows_the_sum() {
    let out = run(
        "fill",
        None,
        &format!(
            r#"{SHOW}{DESIGN}
Show::rows(Template::open($template)->fill([
    "number" => 7,
    "customer" => ["name" => "Shop"],
    "items" => [["name" => "Pen", "qty" => 2], ["name" => "Ink", "qty" => 1]],
])->bytes());
"#
        ),
    );
    assert_eq!(
        out,
        "1 A=\"Invoice 7\" B=\"Shop\"\n2 A=\"Pen\" B=2 C==B2*2\n3 A=\"Ink\" B=1 C==B3*2\n4 A=\"Sum\" B==SUM(B2:B3)\n"
    );
}

/// `set` writes a cell that has no placeholder, a string beginning `=` stays text, and an address
/// on a sheet the template does not have throws a `LogicError`.
// covers: Novis\Spreadsheet\Template::set
#[test]
fn set_writes_one_cell_by_address() {
    let out = run(
        "set",
        None,
        &format!(
            r#"{SHOW}{DESIGN}
var $filling = Template::open($template)->set("Invoice!C1", "=1+1")->set("Invoice!D1", true);
try {{
    $filling->set("Orders!A1", 1);
}} catch (LogicError $e) {{
    echo $e->message, "\n";
}}
Show::rows($filling->fill(["number" => 7, "customer" => ["name" => "Shop"], "items" => []])->bytes());
"#
        ),
    );
    assert_eq!(
        out,
        "The template has no sheet `Orders`, which `Orders!A1` names.\n1 A=\"Invoice 7\" B=\"Shop\" C=\"=1+1\" D=true\n2 A=\"Sum\" B==SUM(B2:B2)\n"
    );
}

/// A template opened from a file the program opened fills the same bytes as one opened from its
/// bytes, and bytes that are not a workbook throw a `ParseError`.
// covers: Novis\Spreadsheet\Template::open
#[test]
fn a_file_and_its_bytes_fill_the_same_and_text_is_not_a_template() {
    let out = run(
        "open",
        Some("[capabilities.fs]\nread = [\".\"]\nwrite = [\".\"]\n"),
        &format!(
            r#"{DESIGN}
array<mixed> $data = ["number" => 7, "customer" => ["name" => "Shop"], "items" => [["name" => "Pen", "qty" => 2]]];
var $saved = Workbook::createForFile("template.xlsx");
$saved->sheet("Invoice")->addAll([
    ["Invoice {{{{number}}}}", "{{{{customer.name}}}}"],
    ["{{{{items.name}}}}", "{{{{items.qty}}}}", Formula::of("B2*2")],
    ["Sum", Formula::of("SUM(B2:B2)")],
]);
$saved->finish();
Core\IO\File $file = Core\IO::open("template.xlsx", Core\IO::FileMode::Read);
bytes $fromFile = Template::open($file)->fill($data)->bytes();
$file->close();
echo $fromFile == Template::open($template)->fill($data)->bytes() ? "the same bytes" : "different bytes", "\n";
try {{
    Template::open("Invoice {{{{number}}}}" as bytes);
}} catch (ParseError $e) {{
    echo "not a workbook\n";
}}
"#
        ),
    );
    assert_eq!(out, "the same bytes\nnot a workbook\n");
}

/// A `Row` made with `new Row` keeps its number and its values, as the one `Reader::rows` makes
/// with `header: true` keys its values by the header text.
// covers: Novis\Spreadsheet\Row::constructor
#[test]
fn a_row_keeps_the_values_it_is_made_from() {
    let out = run(
        "row",
        None,
        r#"
var $row = new Row(3, ["order" => 1042, "total" => 19.5]);
echo $row->number, " ", Core\Json::encode($row->values), "\n";
bytes $file = Workbook::fromRows([["order" => 1042, "total" => 19.5]], {sheet: "Orders"})->bytes();
foreach (Reader::open($file)->rows("Orders", {header: true}) as Row $read) {
    echo $read->number, " ", Core\Json::encode($read->values), "\n";
}
"#,
    );
    assert_eq!(
        out,
        "3 {\"order\":1042,\"total\":19.5}\n2 {\"order\":1042,\"total\":19.5}\n"
    );
}
