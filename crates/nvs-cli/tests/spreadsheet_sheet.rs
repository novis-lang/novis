//! `Novis\Spreadsheet\Sheet`, run through the built binary: rows are added in order, the parts a
//! sheet writes after its rows read back as they were given, a finished sheet throws for every
//! method, and a CSV file writes the rows and none of the parts, as
//! `rule:core-classes/spreadsheet-sheet-end-parts` places them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("spreadsheet-sheet")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `body` after the `use` lines in the case's own directory and returns its stdout, failing
/// the test on a non-zero exit.
fn run(case: &str, body: &str) -> String {
    let dir = scratch(case);
    let program = format!(
        "<?nvs\nuse Novis\\Spreadsheet\\Chart;\nuse Novis\\Spreadsheet\\Formula;\nuse Novis\\Spreadsheet\\Reader;\nuse Novis\\Spreadsheet\\Style;\nuse Novis\\Spreadsheet\\Workbook;\n{body}"
    );
    fs::write(dir.join("main.nvs"), program).unwrap();
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

/// The sheet `Workbook::addSheet` makes adds keyed rows in order, `cell` and `column` address them,
/// and a merged range, a link, a note, a table and an image added after the rows read back.
// covers: Novis\Spreadsheet\Sheet::constructor, Novis\Spreadsheet\Sheet::addRow, Novis\Spreadsheet\Sheet::addRows, Novis\Spreadsheet\Sheet::cell, Novis\Spreadsheet\Sheet::column, Novis\Spreadsheet\Sheet::mergeCells, Novis\Spreadsheet\Sheet::addLink, Novis\Spreadsheet\Sheet::addNote, Novis\Spreadsheet\Sheet::addTable, Novis\Spreadsheet\Sheet::addImage
#[test]
fn rows_are_added_in_order_and_the_parts_read_back() {
    let out = run(
        "parts",
        r##"
bytes $png = Core\Encoding::fromBase64("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==");
var $book = Workbook::createInMemory();
var $sheet = $book->addSheet("Shop");
$sheet->addRow(["item" => "Pen", "qty" => 3], {height: 20.0})->addRows([["item" => "Ink", "qty" => 5], ["item" => "Pad", "qty" => 7]]);
echo $sheet->cell("qty", 4), " ", $sheet->column("qty"), "\n";
$sheet->addTable("A1:B4", {name: "Stock"})
    ->mergeCells("D1:E1")
    ->addLink("A2", "https://example.com/pen", {text: "Pen"})
    ->addNote("B3", "Low.", {author: "Ann"})
    ->addImage("F1", $png);
var $written = $book->finish();
var $reader = Reader::open($written->bytes ?? Core\Bytes::fill(0, 0));
echo "rows ", $written->rows["Shop"], "\n";
foreach ($reader->rows("Shop", {header: true}) as var $row) {
    echo $row->number, " ", $row->values["item"], " ", $row->values["qty"], "\n";
}
foreach ($reader->tables("Shop") as var $table) { echo "table ", $table->name, " ", $table->table, "\n"; }
foreach ($reader->merged("Shop") as var $range) { echo "merged ", $range, "\n"; }
foreach ($reader->links("Shop") as var $link) { echo "link ", $link->anchor, " ", $link->target, "\n"; }
foreach ($reader->notes("Shop") as var $note) { echo "note ", $note->noted, " ", $note->author ?? "none", "\n"; }
foreach ($reader->images("Shop") as var $image) { echo "image ", $image->at, " ", $image->format, "\n"; }
"##,
    );
    assert_eq!(
        out,
        "B4 B2:B4\nrows 3\n2 Pen 3\n3 Ink 5\n4 Pad 7\ntable Stock A1:B4\nmerged D1:E1\nlink A2 https://example.com/pen\nnote B3 Ann\nimage F1 png\n"
    );
}

/// Once the workbook starts the next sheet, every method of the sheet before it that writes or
/// reads its rows throws `LogicError`, and the next sheet is written in full.
// covers: Novis\Spreadsheet\Sheet::constructor, Novis\Spreadsheet\Sheet::addRow, Novis\Spreadsheet\Sheet::addRows, Novis\Spreadsheet\Sheet::column, Novis\Spreadsheet\Sheet::mergeCells, Novis\Spreadsheet\Sheet::addLink, Novis\Spreadsheet\Sheet::addNote, Novis\Spreadsheet\Sheet::addValidation, Novis\Spreadsheet\Sheet::addOutlineGroup, Novis\Spreadsheet\Sheet::addTable, Novis\Spreadsheet\Sheet::addConditionalFormat, Novis\Spreadsheet\Sheet::addImage, Novis\Spreadsheet\Sheet::addChart, Novis\Spreadsheet\Sheet::setPrintOptions, Novis\Spreadsheet\Sheet::protect
#[test]
fn a_finished_sheet_throws_for_every_method() {
    let out = run(
        "finished",
        r##"
bytes $png = Core\Encoding::fromBase64("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==");
var $book = Workbook::createInMemory();
var $first = $book->addSheet("Shop");
$first->addRow(["item" => "Pen", "qty" => 3]);
$book->addSheet("Blog")->addRow(["post" => 1]);
array<callable(): mixed> $calls = [
    fn(): mixed => $first->addRow(["item" => "Ink"]),
    fn(): mixed => $first->addRows([["item" => "Ink"]]),
    fn(): mixed => $first->column("qty"),
    fn(): mixed => $first->mergeCells("A5:B5"),
    fn(): mixed => $first->addLink("A2", "https://example.com"),
    fn(): mixed => $first->addNote("A2", "Late."),
    fn(): mixed => $first->addValidation("B2:B9", {list: ["a"]}),
    fn(): mixed => $first->addOutlineGroup("8:9"),
    fn(): mixed => $first->addTable("A1:B2"),
    fn(): mixed => $first->addConditionalFormat("B2:B9", {kind: "colorScale", colors: ["#FF0000", "#00FF00"]}),
    fn(): mixed => $first->addImage("D1", $png),
    fn(): mixed => $first->addChart("D1", Chart::column()->withSeries("B2:B2")),
    fn(): mixed => $first->setPrintOptions({orientation: "landscape"}),
    fn(): mixed => $first->protect(),
];
uint $thrown = 0;
foreach ($calls as var $call) {
    try {
        $call();
    } catch (LogicError $e) {
        $thrown = $thrown + 1;
    }
}
echo $thrown, " of ", Core\Arr::count($calls), " threw\n";
echo Core\Json::encode($book->finish()->rows), "\n";
"##,
    );
    assert_eq!(out, "14 of 14 threw\n{\"Shop\":1,\"Blog\":1}\n");
}

/// A validation rule writes its kind, its limits and its messages, a rule with two kinds throws,
/// and a conditional format writes its operator, its value and its colours in the order added.
// covers: Novis\Spreadsheet\Sheet::addValidation, Novis\Spreadsheet\Sheet::addConditionalFormat
#[test]
fn validation_and_conditional_formats_are_written_as_given() {
    let out = run(
        "rules",
        r##"
var $book = Workbook::createInMemory();
var $sheet = $book->addSheet("Orders");
$sheet->addRows([["status" => "open", "qty" => 3], ["status" => "paid", "qty" => 12]]);
$sheet->addValidation("A2:A9", {list: ["open", "paid"]})
    ->addValidation("B2:B9", {decimal: {max: 99.5}, error: "Too many."})
    ->addConditionalFormat("B2:B9", {kind: "cell", operator: "between", values: ["1", "5"], style: Style::of({bold: true})})
    ->addConditionalFormat("B2:B9", {kind: "colorScale", colors: ["#F8696B", "#63BE7B"]});
try {
    $sheet->addValidation("C2:C9", {list: ["a"], length: {max: 3}});
} catch (LogicError $e) {
    echo $e->message, "\n";
}
try {
    $sheet->addConditionalFormat("C2:C9", {kind: "dataBar", colors: ["#638EC6"], style: Style::of({bold: true})});
} catch (LogicError $e) {
    echo "a data bar with a style throws\n";
}
var $xml = Core\Zip::read($book->finish()->bytes ?? Core\Bytes::fill(0, 0), "xl/worksheets/sheet1.xml") as string;
echo Core\Str::contains($xml, "<dataValidation type=\"list\" allowBlank=\"1\" showInputMessage=\"1\" showErrorMessage=\"1\" sqref=\"A2:A9\"><formula1>\"open,paid\"</formula1></dataValidation>") ? "list" : "no list", "\n";
echo Core\Str::contains($xml, "<dataValidation type=\"decimal\" operator=\"lessThanOrEqual\"") ? "decimal at most" : "no decimal", "\n";
echo Core\Str::contains($xml, "error=\"Too many.\"") ? "error message" : "no error message", "\n";
echo Core\Str::contains($xml, "<cfRule type=\"cellIs\" dxfId=\"0\" priority=\"1\" operator=\"between\"><formula>1</formula><formula>5</formula></cfRule>") ? "between 1 and 5" : "no between", "\n";
echo Core\Str::contains($xml, "<cfRule type=\"colorScale\" priority=\"2\">") ? "colour scale second" : "no colour scale", "\n";
"##,
    );
    assert_eq!(
        out,
        "The validation of `C2:C9` needs one of `list`, `whole`, `decimal`, `date`, `length` or `formula`. It has 2.\na data bar with a style throws\nlist\ndecimal at most\nerror message\nbetween 1 and 5\ncolour scale second\n"
    );
}

/// A group of columns before the first row hides them when collapsed, a group of rows before its
/// rows sets their level and one after them throws, the page setup and print titles are written,
/// and protection keeps a hash of the password and unlocks what `allow` names.
// covers: Novis\Spreadsheet\Sheet::addOutlineGroup, Novis\Spreadsheet\Sheet::setPrintOptions, Novis\Spreadsheet\Sheet::protect
#[test]
fn groups_page_setup_and_protection_are_written() {
    let out = run(
        "layout",
        r##"
var $book = Workbook::createInMemory();
var $sheet = $book->addSheet("Sales");
$sheet->setPrintOptions({orientation: "portrait", fitToPage: {width: 1, height: 2}, repeatRows: "1:1"})
    ->addOutlineGroup("C:D", {collapsed: true})
    ->addOutlineGroup("3:4");
$sheet->addRows([["shop" => "North", "a" => 1, "b" => 2], ["shop" => "South", "a" => 3, "b" => 4], ["shop" => "East", "a" => 5, "b" => 6]]);
try {
    $sheet->addOutlineGroup("2:3");
} catch (LogicError $e) {
    echo "a late group throws\n";
}
$sheet->protect({password: "secret", allow: ["sort"]});
bytes $file = $book->finish()->bytes ?? Core\Bytes::fill(0, 0);
var $xml = Core\Zip::read($file, "xl/worksheets/sheet1.xml") as string;
var $workbook = Core\Zip::read($file, "xl/workbook.xml") as string;
echo Core\Str::contains($xml, "hidden=\"1\" outlineLevel=\"1\"/>") ? "columns hidden" : "columns shown", "\n";
echo Core\Str::contains($xml, "<row r=\"3\" outlineLevel=\"1\">") ? "row 3 grouped" : "row 3 not grouped", "\n";
echo Core\Str::contains($xml, "<row r=\"2\">") ? "row 2 not grouped" : "row 2 grouped", "\n";
echo Core\Str::contains($xml, "<pageSetup orientation=\"portrait\" fitToWidth=\"1\" fitToHeight=\"2\"/>") ? "page setup" : "no page setup", "\n";
echo Core\Str::contains($workbook, "_xlnm.Print_Titles") ? "print titles" : "no print titles", "\n";
echo Core\Str::contains($xml, "<sheetProtection password=") ? "protected" : "open", "\n";
echo Core\Str::contains($xml, "secret") ? "password kept" : "password not kept", "\n";
echo Core\Str::contains($xml, "sort=\"0\"") ? "sort allowed" : "sort locked", "\n";
"##,
    );
    assert_eq!(
        out,
        "a late group throws\ncolumns hidden\nrow 3 grouped\nrow 2 not grouped\npage setup\nprint titles\nprotected\npassword not kept\nsort allowed\n"
    );
}

/// A CSV file writes the rows `addRow` and `addRows` give, and each part a sheet adds after its rows
/// writes nothing to it. `column` still gives the data rows' range, counted under the header row.
// covers: Novis\Spreadsheet\Sheet::addRow, Novis\Spreadsheet\Sheet::addRows, Novis\Spreadsheet\Sheet::column, Novis\Spreadsheet\Sheet::mergeCells, Novis\Spreadsheet\Sheet::addLink, Novis\Spreadsheet\Sheet::addNote, Novis\Spreadsheet\Sheet::addValidation, Novis\Spreadsheet\Sheet::addOutlineGroup, Novis\Spreadsheet\Sheet::addConditionalFormat, Novis\Spreadsheet\Sheet::addChart, Novis\Spreadsheet\Sheet::setPrintOptions, Novis\Spreadsheet\Sheet::protect
#[test]
fn a_csv_file_writes_the_rows_and_none_of_the_parts() {
    let out = run(
        "csv",
        r##"
var $book = Workbook::createInMemory({format: "csv"});
var $sheet = $book->addSheet("Shop");
$sheet->setPrintOptions({orientation: "landscape"})->addOutlineGroup("3:3");
$sheet->addRow(["item" => "Pen", "qty" => 3])->addRows([["item" => "Ink", "qty" => 5]]);
$sheet->mergeCells("A1:B1")
    ->addLink("A2", "https://example.com")
    ->addNote("A2", "Low.")
    ->addValidation("B2:B9", {whole: {min: 1}})
    ->addConditionalFormat("B2:B9", {kind: "unique", style: Style::of({bold: true})})
    ->addChart("D2", Chart::column()->withSeries($sheet->column("qty")))
    ->protect({password: "secret"});
string $range = $sheet->column("qty");
echo "{$range}\n";
echo ($book->finish()->bytes ?? Core\Bytes::fill(0, 0)) as string;
"##,
    );
    assert_eq!(out, "B2:B3\n\u{feff}item,qty␍\nPen,3␍\nInk,5␍\n");
}
