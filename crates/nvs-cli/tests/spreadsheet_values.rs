//! `Novis\Spreadsheet\Export`, `Style`, `Cell` and `Formula`, the value classes of the built-in
//! spreadsheet writer, run through the built binary: an export writes the same file through each
//! of its methods, and a style, a cell and a formula land in the written bytes, as
//! `rule:core-classes/spreadsheet-writes-as-a-stream`,
//! `rule:core-classes/spreadsheet-styles-and-defaults` and
//! `rule:core-classes/spreadsheet-formula-is-a-value` place them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("spreadsheet-values")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `body` after the `use` lines in the case's directory, with `extra` arguments before the
/// program, and returns its stdout, failing the test on a non-zero exit.
fn run_in(dir: &Path, extra: &[&str], body: &str) -> String {
    String::from_utf8(bytes_in(dir, extra, body)).expect("the output is UTF-8")
}

/// As [`run_in`], and returns stdout as bytes, for a program that writes a binary body.
fn bytes_in(dir: &Path, extra: &[&str], body: &str) -> Vec<u8> {
    let program = format!(
        "<?nvs\nuse Novis\\Spreadsheet\\Cell;\nuse Novis\\Spreadsheet\\Export;\nuse Novis\\Spreadsheet\\Formula;\nuse Novis\\Spreadsheet\\Reader;\nuse Novis\\Spreadsheet\\Style;\nuse Novis\\Spreadsheet\\Template;\nuse Novis\\Spreadsheet\\Workbook;\n{body}"
    );
    fs::write(dir.join("main.nvs"), program).unwrap();
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("run")
        .args(extra)
        .arg("main.nvs")
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(
        out.status.success(),
        "the program runs: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// Runs `body` with no `nvs.toml` and no request.
fn run(case: &str, body: &str) -> String {
    run_in(&scratch(case), &[], body)
}

/// Runs `body` in a directory whose `nvs.toml` grants reading and writing that directory.
fn run_with_files(case: &str, body: &str) -> (PathBuf, String) {
    let dir = scratch(case);
    fs::write(
        dir.join("nvs.toml"),
        "[capabilities.fs]\nread = [\".\"]\nwrite = [\".\"]\n",
    )
    .unwrap();
    let out = run_in(&dir, &[], body);
    (dir, out)
}

/// `new Export` runs its function once for each method, and `bytes`, `save` and `to` write the
/// same file; `save` and `to` count the rows of each sheet and keep no bytes.
// covers: Novis\Spreadsheet\Export::constructor, Novis\Spreadsheet\Export::bytes, Novis\Spreadsheet\Export::save, Novis\Spreadsheet\Export::to
#[test]
fn an_export_writes_the_same_file_through_bytes_save_and_to() {
    let (dir, out) = run_with_files(
        "export-targets",
        r#"
class Shop {
    public static int $runs = 0;
}
var $export = new Export(fn (Workbook $book): void => {
    Shop::$runs = Shop::$runs + 1;
    $book->sheet("Orders")->addAll([["order" => 1042, "total" => 19.5], ["order" => 1043, "total" => 5.0]]);
    $book->sheet("Returns")->add(["order" => 1042]);
}, null);
Core\IO\File $memory = Core\IO::open("memory.xlsx", Core\IO::FileMode::Write);
$memory->write($export->bytes());
$memory->close();
var $saved = $export->save("saved.xlsx");
Core\IO\File $open = Core\IO::open("open.xlsx", Core\IO::FileMode::Write);
var $streamed = $export->to($open);
$open->write("");
$open->close();
echo Shop::$runs, " runs\n";
echo $saved->rows["Orders"], " ", $saved->rows["Returns"], " ", $saved->bytes == null ? "no bytes" : "bytes", "\n";
echo $streamed->rows["Orders"], " ", $streamed->size == $saved->size ? "same size" : "other size", "\n";
"#,
    );
    assert_eq!(out, "3 runs\n2 1 no bytes\n2 same size\n");
    let memory = fs::read(dir.join("memory.xlsx")).unwrap();
    assert!(memory.starts_with(b"PK"), "an xlsx file is a zip archive");
    assert_eq!(fs::read(dir.join("saved.xlsx")).unwrap(), memory);
    assert_eq!(fs::read(dir.join("open.xlsx")).unwrap(), memory);
}

/// `save` to a `.csv` path writes CSV, and `to` writes from the open file's position and leaves it
/// open, for an export and for a filled template alike.
// covers: Novis\Spreadsheet\Export::save, Novis\Spreadsheet\Export::to
#[test]
fn save_reads_the_format_from_the_path_and_to_writes_where_the_file_is() {
    let (dir, out) = run_with_files(
        "export-csv-and-template",
        r##"
Workbook::fromRows([["name" => "Ada", "city" => "London"]])->save("people.csv");
Core\IO\File $log = Core\IO::open("log.csv", Core\IO::FileMode::Write);
$log->write("# orders\n");
Workbook::fromRows([["order" => 1]], {format: "csv"})->to($log);
$log->write("# end\n");
$log->close();
var $design = Workbook::createInMemory();
$design->sheet("Invoice")->add(["Invoice for {{shop}}"]);
bytes $template = $design->finish()->bytes ?? Core\Bytes::fill(0, 0);
var $written = Template::open($template)->fill(["shop" => "Shop"])->save("invoice.xlsx");
echo $written->rows["Invoice"], " ", $written->bytes == null ? "no bytes" : "bytes", "\n";
foreach (Reader::open(Core\IO::open("invoice.xlsx", Core\IO::FileMode::Read))->rows("Invoice") as var $row) {
    echo Core\Json::encode($row->values), "\n";
}
"##,
    );
    assert_eq!(out, "1 no bytes\n{\"A\":\"Invoice for Shop\"}\n");
    assert_eq!(
        fs::read_to_string(dir.join("people.csv")).unwrap(),
        "name,city\nAda,London\n"
    );
    assert_eq!(
        fs::read_to_string(dir.join("log.csv")).unwrap(),
        "# orders\norder\n1\n# end\n"
    );
}

/// `download` answers a request with the export as its body, and a filled template sends the
/// same bytes `bytes` returns.
// covers: Novis\Spreadsheet\Export::download
#[test]
fn a_download_sends_the_export_as_the_body() {
    let dir = scratch("export-download");
    fs::write(
        dir.join("request.nvsr"),
        "--METHOD--\nGET\n--PATH--\n/orders.csv\n--HEADERS--\nhost: example.com\n",
    )
    .unwrap();
    let out = run_in(
        &dir,
        &["--request", "request.nvsr"],
        r#"
Workbook::fromRows([["order" => 1042, "note" => "=1+2"]])->download("orders.csv");
"#,
    );
    assert_eq!(out, "order,note\n1042,'=1+2\n");

    // A workbook is not UTF-8, so `Core\Out::capture` cannot read it back. The program writes
    // the expected bytes and then the download, and the two halves of its output must match.
    let out = bytes_in(
        &dir,
        &["--request", "request.nvsr"],
        r#"
var $design = Workbook::createInMemory();
$design->sheet("Invoice")->add(["{{shop}}"]);
bytes $template = $design->finish()->bytes ?? Core\Bytes::fill(0, 0);
Core\Response::bytes(Template::open($template)->fill(["shop" => "Shop"])->bytes(), "application/octet-stream");
Template::open($template)->fill(["shop" => "Shop"])->download("Rechnung März \"1\".xlsx");
"#,
    );
    let (expected, sent) = out.split_at(out.len() / 2);
    assert!(!expected.is_empty(), "the program wrote the workbook");
    assert_eq!(expected, sent, "the download sends the filled workbook");
}

/// A style's fields land in `xl/styles.xml` as one cell format, equal fields give equal keys, a
/// name that is not allowed throws a `LogicError`, and `new Style` keeps the values it is made from.
// covers: Novis\Spreadsheet\Style::of, Novis\Spreadsheet\Style::constructor
#[test]
fn a_style_lands_in_the_styles_part_once() {
    let out = run(
        "style",
        r##"
var $total = Style::of({bold: true, border: "thin", format: "money", align: "right", fill: "#FFFF00"});
var $book = Workbook::createInMemory();
var $sheet = $book->sheet("Shop", {header: false});
$sheet->add(["Pen", 1.5], {style: $total});
$sheet->add(["Ink", 4.25], {style: Style::of({bold: true, border: "thin", format: "money", align: "right", fill: "#FFFF00"})});
bytes $file = $book->finish()->bytes ?? Core\Bytes::fill(0, 0);
var $styles = Core\Zip::read($file, "xl/styles.xml") as string;
array<string> $parts = ["<b/>", "<left style=\"thin\">", "<fgColor rgb=\"FFFFFF00\"/>", "numFmtId=\"4\"", "<alignment horizontal=\"right\"/>", "<cellXfs count=\"2\">"];
foreach ($parts as string $part) {
    echo Core\Str::contains($styles, $part) ? "has " : "lacks ", $part, "\n";
}
echo Style::of({bold: true, size: 12.0})->key == Style::of({bold: true, size: 12.0})->key ? "equal keys" : "other keys", "\n";
echo Style::of({bold: true})->key == Style::of({italic: true})->key ? "equal keys" : "other keys", "\n";
try {
    Style::of({underline: "triple"});
} catch (LogicError $e) {
    echo "LogicError\n";
}
var $kept = new Style($total->fields, "total");
echo $kept->key, " ", $kept->fields->bold == true ? "bold" : "plain", " ", $kept->fields->italic == null ? "no italic" : "italic", "\n";
"##,
    );
    assert_eq!(
        out,
        "has <b/>\nhas <left style=\"thin\">\nhas <fgColor rgb=\"FFFFFF00\"/>\nhas numFmtId=\"4\"\nhas <alignment horizontal=\"right\"/>\nhas <cellXfs count=\"2\">\nequal keys\nother keys\nLogicError\ntotal bold no italic\n"
    );
}

/// A cell's own style wins field by field over its row's, the cell is written as its value, CSV
/// leaves the style out, and `new Cell` keeps the values it is made from.
// covers: Novis\Spreadsheet\Cell::of, Novis\Spreadsheet\Cell::constructor
#[test]
fn a_cell_style_wins_over_its_row_field_by_field() {
    let out = run(
        "cell",
        r#"
var $book = Workbook::createInMemory();
$book->sheet("Sales", {header: false})->add(["Pens", 1249.5, Cell::of(0.25, Style::of({format: "percent"}))], {style: Style::of({format: "money", bold: true})});
bytes $file = $book->finish()->bytes ?? Core\Bytes::fill(0, 0);
foreach (Reader::open($file)->rows("Sales", {text: true}) as var $row) {
    echo Core\Json::encode($row->values), "\n";
}
var $styles = Core\Zip::read($file, "xl/styles.xml") as string;
echo Core\Str::contains($styles, "<xf numFmtId=\"9\" fontId=\"1\"") ? "percent and bold" : "another style", "\n";
echo Workbook::fromRows([["item" => Cell::of("Pen", Style::of({bold: true})), "price" => 1.5]], {format: "csv"})->bytes() as string;
var $cell = new Cell(1042, Style::of({italic: true}));
echo $cell->value, " ", $cell->style->fields->italic == true ? "italic" : "plain", "\n";
"#,
    );
    assert_eq!(
        out,
        "{\"A\":\"Pens\",\"B\":\"1,249.50\",\"C\":\"25%\"}\npercent and bold\nitem,price\nPen,1.5\n1042 italic\n"
    );
}

/// A rich cell lands as one inline cell of runs, each run's font its own `<rPr>`, the row's fill
/// stays on the cell, the reader gives the runs' text joined, and a run's fill throws.
// covers: Novis\Spreadsheet\Cell::rich
#[test]
fn a_rich_cell_writes_each_run_with_its_own_font() {
    let out = run(
        "rich",
        r##"
var $cell = Cell::rich([{text: "Due "}, {text: "today", style: Style::of({bold: true, underline: true})}]);
var $book = Workbook::createInMemory();
$book->sheet("Tasks", {header: false})->add([$cell], {style: Style::of({fill: "#FFFF00"})});
bytes $file = $book->finish()->bytes ?? Core\Bytes::fill(0, 0);
var $sheet = Core\Zip::read($file, "xl/worksheets/sheet1.xml") as string;
echo Core\Str::contains($sheet, "<r><rPr><b/><u/></rPr><t>today</t></r>") ? "bold run" : $sheet, "\n";
var $styles = Core\Zip::read($file, "xl/styles.xml") as string;
echo Core\Str::contains($styles, "<fgColor rgb=\"FFFFFF00\"/>") ? "row fill" : "no fill", "\n";
foreach (Reader::open($file)->rows("Tasks") as var $row) {
    echo $row->values["A"], "\n";
}
try {
    Cell::rich([{text: "x", style: Style::of({align: "center"})}]);
} catch (LogicError $e) {
    echo $e->message, "\n";
}
"##,
    );
    assert_eq!(
        out,
        "bold run\nrow fill\nDue today\nPart 1 of `Cell::rich` has a style with `align`. A part can only set font fields: `font`, `size`, `bold`, `italic`, `strike`, `underline` and `color`.\n"
    );
}

/// A `Formula` lands as a formula cell with its cached value and no `=`, a string that starts
/// with `=` lands as a text cell, the reader gives the formula back, and `new Formula` keeps the
/// values it is made from.
// covers: Novis\Spreadsheet\Formula::of, Novis\Spreadsheet\Formula::constructor
#[test]
fn only_a_formula_value_is_written_as_a_formula() {
    let out = run(
        "formula",
        r#"
tainted string $typed = "=1+1";
bytes $file = Workbook::fromRows([["total" => Formula::of("=SUM(B1:B1)", {cached: 1249.5}), "note" => $typed]], {header: false})->bytes();
var $sheet = Core\Zip::read($file, "xl/worksheets/sheet1.xml") as string;
echo Core\Str::contains($sheet, "<c r=\"A1\"><f>SUM(B1:B1)</f><v>1249.5</v></c>") ? "formula cell" : "no formula cell", "\n";
echo Core\Str::contains($sheet, "<c r=\"B1\" t=\"inlineStr\"><is><t>=1+1</t></is></c>") ? "text cell" : "no text cell", "\n";
foreach (Reader::open($file)->rows("Sheet1", {formulas: true}) as var $row) {
    mixed $total = $row->values["A"];
    if ($total is Formula) {
        echo Core\Taint::assertTrusted($total->text, "the test wrote it"), " = ", $total->cached, "\n";
    }
}
var $plain = Formula::of("A1*2");
echo $plain->cached == null ? "no cached value" : "a cached value", "\n";
var $made = new Formula("B2*C2", "ten");
echo Core\Taint::assertTrusted($made->text, "the test wrote it"), " ", $made->cached, "\n";
"#,
    );
    assert_eq!(
        out,
        "formula cell\ntext cell\nSUM(B1:B1) = 1249.5\nno cached value\nB2*C2 ten\n"
    );
}
