//! `Novis\Spreadsheet\Engine`, the spreadsheet component's manifest class, called directly from a
//! program through the built binary: the `writer` exports write a workbook that the `reader` exports
//! read back field by field, the `template` exports fill a template, and a call out of order throws
//! a `LogicError` from the export itself, with no `Workbook`, `Reader` or `Template` in between.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("spreadsheet-engine")
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
        "<?nvs\nuse Novis\\Spreadsheet\\Engine;\nuse Novis\\Spreadsheet\\Filling;\nuse Novis\\Spreadsheet\\Reader;\nuse Novis\\Spreadsheet\\Reading;\nuse Novis\\Spreadsheet\\Visibility;\nuse Novis\\Spreadsheet\\Workbook;\nuse Novis\\Spreadsheet\\Writer;\n{body}"
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

/// The options of a sheet named `Shop` with nothing else set, and the read options of that sheet
/// with the header in `headerRow`, as `Workbook::sheet` and `Reader::rows` build them.
const SHAPES: &str = r#"
class Shop
{
    type Layout = {name: string, position: ?uint, visibility: Visibility, columns: array<{width: ?float, auto: bool, style: ?uint, format: ?string, hidden: bool}>, freezeRows: uint, freezeColumns: uint, autofilter: ?string, tabColor: ?string, zoom: ?uint, gridlines: bool, rightToLeft: bool, header: {defaultStyle: bool, style: ?uint, freeze: bool, filter: bool}, newSheetOnOverflow: bool};
    type Asked = {sheet: string, headerRow: ?uint, from: ?uint, limit: ?uint, columns: array<string>, text: array<string>, textAll: bool, formulas: bool, keepEmptyRows: bool, fillMerged: bool, skipHidden: bool, locale: string};

    public static function layout(): Layout
    {
        return {name: 'Shop', position: null, visibility: Visibility::Visible, columns: [], freezeRows: 0, freezeColumns: 0, autofilter: null, tabColor: null, zoom: null, gridlines: true, rightToLeft: false, header: {defaultStyle: false, style: null, freeze: false, filter: false}, newSheetOnOverflow: false};
    }

    public static function asked(?uint $headerRow, ?uint $from): Asked
    {
        return {sheet: 'Shop', headerRow: $headerRow, from: $from, limit: null, columns: [], text: [], textAll: false, formulas: true, keepEmptyRows: false, fillMerged: false, skipHidden: false, locale: 'en'};
    }
}
"#;

/// The `writer` exports write a styled header, rows, a formula over `writerColumn`'s range, a
/// merge, a link, a note and a defined name; `writerFinish` counts the rows and gives the size of
/// the joined bytes; and each `reader` export returns what was written, the header from
/// `readerHeader` and the rows in two batches from `readerRows`.
// covers: Novis\Spreadsheet\Engine::writerOpen, Novis\Spreadsheet\Engine::writerStyle, Novis\Spreadsheet\Engine::writerSheet, Novis\Spreadsheet\Engine::writerRows, Novis\Spreadsheet\Engine::writerPart, Novis\Spreadsheet\Engine::writerDefine, Novis\Spreadsheet\Engine::writerCell, Novis\Spreadsheet\Engine::writerColumn, Novis\Spreadsheet\Engine::writerFinish
// covers: Novis\Spreadsheet\Engine::readerOpen, Novis\Spreadsheet\Engine::readerSheets, Novis\Spreadsheet\Engine::readerMacros, Novis\Spreadsheet\Engine::readerMerged, Novis\Spreadsheet\Engine::readerLinks, Novis\Spreadsheet\Engine::readerNotes, Novis\Spreadsheet\Engine::readerNames, Novis\Spreadsheet\Engine::readerTables, Novis\Spreadsheet\Engine::readerImages, Novis\Spreadsheet\Engine::readerProperties, Novis\Spreadsheet\Engine::readerHeader, Novis\Spreadsheet\Engine::readerRows
#[test]
fn the_writer_exports_write_what_the_reader_exports_read_back() {
    let body = [
        SHAPES,
        r#"
Writer $writer = Engine::writerOpen({properties: {title: 'Prices', subject: null, author: 'Shop', company: null, keywords: null, created: null}, target: 'prices.xlsx'});
uint $bold = Engine::writerStyle($writer, {font: null, size: null, bold: true, italic: null, strike: null, underline: null, color: null, fill: null, border: null, borderColor: null, align: null, valign: null, wrap: null, rotate: null, indent: null, format: null, locked: null, hideFormula: null});
array<bytes> $parts = [];
$parts[] = Engine::writerSheet($writer, Shop::layout(), [{cells: [{value: {text: 'Item'}, style: $bold}, {value: {text: 'Price'}, style: $bold}], style: null, height: null, hidden: false, level: 0, header: true}]);
$parts[] = Engine::writerRows($writer, [
    {cells: [{value: {text: 'Widget'}, style: null}, {value: {number: 2.5}, style: null}], style: null, height: null, hidden: false, level: 0, header: false},
    {cells: [{value: {text: 'Gadget'}, style: null}, {value: {number: 4.0}, style: null}], style: null, height: null, hidden: false, level: 0, header: false},
]);
string $prices = Engine::writerColumn($writer, 1);
echo Engine::writerCell($writer, 1, 4), ' ', $prices, "\n";
$parts[] = Engine::writerRows($writer, [{cells: [{value: {text: 'Sum'}, style: null}, {value: {formula: 'SUM(' . $prices . ')', cachedText: null, cachedNumber: 6.5}, style: null}], style: null, height: null, hidden: false, level: 0, header: false}]);
$parts[] = Engine::writerPart($writer, {merge: 'C1:D1'});
$parts[] = Engine::writerPart($writer, {anchor: 'A2', target: 'https://example.com/widget', text: 'Widget', tooltip: null});
$parts[] = Engine::writerPart($writer, {noted: 'A3', note: 'Check this price.', author: 'Ann'});
Engine::writerDefine($writer, 'Total', 'Shop!$B$4');
var $last = Engine::writerFinish($writer);
$parts[] = $last->data;
bytes $file = Core\Bytes::join($parts);
echo $last->rows[0]->name, ' ', $last->rows[0]->rows, ' ', $last->size == Core\Bytes::length($file) ? 'size matches' : 'size differs', "\n";

Reading $reader = Engine::readerOpen($file, {maxBytes: null, maxRatio: null});
foreach (Engine::readerSheets($reader, $file) as var $sheet) {
    echo $sheet->position, ' ', $sheet->name, "\n";
}
echo Core\Json::encode(Engine::readerHeader($reader, $file, Shop::asked(1, null))), "\n";
var $batch = Engine::readerRows($reader, $file, Shop::asked(1, null), 2);
foreach ($batch->rows as var $row) {
    echo $row->number, ' ', Core\Json::encode($row->cells), "\n";
}
echo $batch->done ? 'done' : 'more', "\n";
var $rest = Engine::readerRows($reader, $file, Shop::asked(1, 4), 10);
foreach ($rest->rows as var $row) {
    echo $row->number, ' ', Core\Json::encode($row->cells), "\n";
}
echo $rest->done ? 'done' : 'more', "\n";
echo Core\Json::encode(Engine::readerMerged($reader, $file, 'Shop')), "\n";
echo Core\Json::encode(Engine::readerLinks($reader, $file, 'Shop')), "\n";
echo Core\Json::encode(Engine::readerNotes($reader, $file, 'Shop')), "\n";
echo Core\Json::encode(Engine::readerNames($reader, $file)), "\n";
echo Core\Json::encode(Engine::readerTables($reader, $file, 'Shop')), Core\Json::encode(Engine::readerImages($reader, $file, 'Shop')), Core\Json::encode(Engine::readerMacros($reader, $file)), "\n";
var $properties = Engine::readerProperties($reader, $file);
echo $properties->title ?? 'null', ' ', $properties->author ?? 'null', ' ', $properties->subject ?? 'null', "\n";
"#,
    ]
    .concat();
    assert_eq!(
        run("write-and-read", &body),
        r#"B4 B2:B3
Shop 3 size matches
0 Shop
["Item","Price"]
2 [{"column":1,"display":null,"formula":null,"value":{"text":"Widget"}},{"column":2,"display":null,"formula":null,"value":{"number":2.5}}]
3 [{"column":1,"display":null,"formula":null,"value":{"text":"Gadget"}},{"column":2,"display":null,"formula":null,"value":{"integer":4}}]
more
4 [{"column":1,"display":null,"formula":null,"value":{"text":"Sum"}},{"column":2,"display":null,"formula":"SUM(B2:B3)","value":{"number":6.5}}]
done
["C1:D1"]
[{"anchor":"A2","external":true,"target":"https://example.com/widget","text":"Widget","tooltip":null}]
[{"author":"Ann","note":"Check this price.","noted":"A3"}]
[{"name":"Total","reference":"Shop!$B$4","sheet":null}]
[][][]
Prices Shop null
"#
    );
}

/// `templateSet` fills a cell by its address, `templateFill` the placeholder outside the list and
/// returns the list, `templateItems` writes one row per item, and the cell `templateSet` filled
/// moves down with the rows the list added.
// covers: Novis\Spreadsheet\Engine::templateOpen, Novis\Spreadsheet\Engine::templateSet, Novis\Spreadsheet\Engine::templateFill, Novis\Spreadsheet\Engine::templateItems, Novis\Spreadsheet\Engine::templateFinish
#[test]
fn the_template_exports_fill_a_template_row_by_row() {
    let out = run(
        "template",
        r#"
var $design = Workbook::createInMemory();
$design->sheet('Shop')->addAll([['{{customer.name}}', 'Total'], ['{{items.name}}', '{{items.qty}}'], ['Sum', '']]);
bytes $template = $design->finish()->bytes ?? Core\Bytes::fill(0, 0);

Filling $filling = Engine::templateOpen($template, {maxBytes: null, maxRatio: null});
Engine::templateSet($filling, 'Shop!B3', {integer: 3});
var $filled = Engine::templateFill($filling, [{path: 'customer.name', value: {text: 'Example Ltd'}}], ['items']);
echo Core\Json::encode($filled->lists), "\n";
array<bytes> $parts = [$filled->data];
$parts[] = Engine::templateItems($filling, {name: 'items', items: [[{path: 'name', value: {text: 'Pen'}}, {path: 'qty', value: {integer: 2}}], [{path: 'name', value: {text: 'Ink'}}, {path: 'qty', value: {integer: 1}}]]});
var $last = Engine::templateFinish($filling, $template);
$parts[] = $last->data;
bytes $file = Core\Bytes::join($parts);
echo $last->rows[0]->name, ' ', $last->rows[0]->rows, ' ', $last->size == Core\Bytes::length($file) ? 'size matches' : 'size differs', "\n";
foreach (Reader::open($file)->rows('Shop') as var $row) {
    echo $row->number, ' ', Core\Json::encode($row->values), "\n";
}
"#,
    );
    assert_eq!(
        out,
        r#"["items"]
Shop 4 size matches
1 {"A":"Example Ltd","B":"Total"}
2 {"A":"Pen","B":2}
3 {"A":"Ink","B":1}
4 {"A":"Sum","B":3}
"#
    );
}

/// Each export called out of order throws a `LogicError` itself: rows with no sheet, a second
/// finish, a sheet the workbook does not have, a source that is not `bytes`, items before the
/// template is filled, and a finish before it.
// covers: Novis\Spreadsheet\Engine::writerRows, Novis\Spreadsheet\Engine::writerFinish, Novis\Spreadsheet\Engine::readerMerged, Novis\Spreadsheet\Engine::readerSheets, Novis\Spreadsheet\Engine::templateItems, Novis\Spreadsheet\Engine::templateFinish
#[test]
fn an_export_called_out_of_order_throws_a_logic_error() {
    let body = [
        SHAPES,
        r#"
class Check {
    public static function attempt(callable(): void $step): string {
        try {
            $step();
            return "ran";
        } catch (LogicError $e) {
            return "LogicError";
        }
    }
}
Writer $empty = Engine::writerOpen({properties: {title: null, subject: null, author: null, company: null, keywords: null, created: null}, target: 'shop.xlsx'});
echo Check::attempt(fn (): void => { Engine::writerRows($empty, []); }), "\n";
Writer $writer = Engine::writerOpen({properties: {title: null, subject: null, author: null, company: null, keywords: null, created: null}, target: 'shop.xlsx'});
array<bytes> $parts = [Engine::writerSheet($writer, Shop::layout(), [])];
$parts[] = Engine::writerFinish($writer)->data;
echo Check::attempt(fn (): void => { Engine::writerFinish($writer); }), "\n";
bytes $file = Core\Bytes::join($parts);
Reading $reader = Engine::readerOpen($file, {maxBytes: null, maxRatio: null});
echo Check::attempt(fn (): void => { Engine::readerMerged($reader, $file, 'Blog'); }), "\n";
echo Check::attempt(fn (): void => { Engine::readerSheets($reader, 'Shop'); }), "\n";
Filling $filling = Engine::templateOpen($file, {maxBytes: null, maxRatio: null});
echo Check::attempt(fn (): void => { Engine::templateItems($filling, {name: 'items', items: []}); }), "\n";
echo Check::attempt(fn (): void => { Engine::templateFinish($filling, $file); }), "\n";
"#,
    ]
    .concat();
    assert_eq!(
        run("out-of-order", &body),
        "LogicError\nLogicError\nLogicError\nLogicError\nLogicError\nLogicError\n"
    );
}
