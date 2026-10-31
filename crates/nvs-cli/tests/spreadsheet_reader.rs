//! `Novis\Spreadsheet\Reader`, run through the built binary: a workbook opened from its bytes or
//! from an open file lists its sheets, reads its rows, and returns its merged cells, links, notes,
//! names, tables, pictures, properties and macro modules, and a CSV file gives an empty list for
//! each of those, as `rule:core-classes/spreadsheet-reads-as-a-stream` places them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("spreadsheet-reader")
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
        "<?nvs\nuse Novis\\Spreadsheet\\Reader;\nuse Novis\\Spreadsheet\\Row;\nuse Novis\\Spreadsheet\\Workbook;\n{body}"
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

/// A workbook with something of every kind `Reader` returns on its sheet `Shop`, and a hidden
/// sheet `Totals`, as `$file`.
const BOOK: &str = r#"
bytes $png = Core\Encoding::fromBase64("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==");
var $book = Workbook::createInMemory({properties: {title: "Prices", author: "Shop", created: Core\Time::fromEpoch(1704067200)}});
var $sheet = $book->sheet("Shop");
$sheet->table("A1:C3", {name: "Prices"});
$sheet->addAll([["Item", "Price", "Qty"], ["Widget", 2.5, 3], ["Gadget", 4.0, 1]]);
$sheet->merge("A5:C5")
    ->link("A2", "https://example.com/widget", {text: "Widget", tooltip: "Open"})
    ->link("B2", "Totals!A1")
    ->note("A3", "Check this price.", {author: "Ann"})
    ->note("B3", "No author.")
    ->image("E2", $png);
$book->sheet("Totals", {hidden: true})->add(["Sum", 6.5]);
$book->define("Rate", "Shop!\$B\$2");
bytes $file = $book->finish()->bytes ?? Core\Bytes::fill(0, 0);
"#;

/// `open` reads the bytes of an xlsx file, `sheets` lists both sheets in tab order with their
/// visibility, and `rows` keys each row by its header with its row number.
// covers: Novis\Spreadsheet\Reader::open, Novis\Spreadsheet\Reader::sheets, Novis\Spreadsheet\Reader::rows
#[test]
fn open_lists_the_sheets_and_reads_the_rows() {
    let body = [
        BOOK,
        r#"
var $reader = Reader::open($file);
foreach ($reader->sheets() as var $info) {
    echo $info->position, " ", $info->name, " ", $info->visibility == Reader::Visibility::Visible ? "visible" : "hidden", "\n";
}
foreach ($reader->rows("Shop", {header: true}) as Row $row) {
    echo $row->number, " ", Core\Json::encode($row->values), "\n";
}
"#,
    ]
    .concat();
    assert_eq!(
        run("open", None, &body),
        "0 Shop visible\n1 Totals hidden\n2 {\"Item\":\"Widget\",\"Price\":2.5,\"Qty\":3}\n3 {\"Item\":\"Gadget\",\"Price\":4,\"Qty\":1}\n"
    );
}

/// `links` returns each link with its cell, target, text, tooltip and whether it leaves the
/// workbook, and `notes` each note with its cell, text and author, `null` when it has none.
// covers: Novis\Spreadsheet\Reader::links, Novis\Spreadsheet\Reader::notes
#[test]
fn links_and_notes_carry_every_field() {
    let body = [
        BOOK,
        r#"
var $reader = Reader::open($file);
echo Core\Json::encode($reader->links("Shop")), "\n";
echo Core\Json::encode($reader->notes(0)), "\n";
"#,
    ]
    .concat();
    assert_eq!(
        run("links-notes", None, &body),
        "[{\"anchor\":\"A2\",\"external\":true,\"target\":\"https://example.com/widget\",\"text\":\"Widget\",\"tooltip\":\"Open\"},{\"anchor\":\"B2\",\"external\":false,\"target\":\"Totals!A1\",\"text\":null,\"tooltip\":null}]\n\
         [{\"author\":\"Ann\",\"note\":\"Check this price.\",\"noted\":\"A3\"},{\"author\":null,\"note\":\"No author.\",\"noted\":\"B3\"}]\n"
    );
}

/// `tables` returns the table's name, range and sheet, `images` the picture's cell, format and
/// the same bytes that were written, and `merged` each merged range.
// covers: Novis\Spreadsheet\Reader::tables, Novis\Spreadsheet\Reader::images, Novis\Spreadsheet\Reader::merged
#[test]
fn tables_images_and_merged_cells_read_back() {
    let body = [
        BOOK,
        r#"
var $reader = Reader::open($file);
echo Core\Json::encode($reader->tables("Shop")), "\n";
foreach ($reader->images("Shop") as var $image) {
    echo $image->at, " ", $image->format, " ", $image->image == $png ? "same" : "different", " ", $image->link ?? "no link", "\n";
}
echo Core\Json::encode($reader->merged("Shop")), "\n";
echo Core\Json::encode($reader->merged("Totals")), "\n";
"#,
    ]
    .concat();
    assert_eq!(
        run("tables-images", None, &body),
        "[{\"name\":\"Prices\",\"sheet\":\"Shop\",\"table\":\"A1:C3\"}]\nE2 png same no link\n[\"A5:C5\"]\n[]\n"
    );
}

/// `names` returns a workbook-wide name with a `null` sheet, `properties` the title, author and
/// creation time written and `null` for the rest, and `macros` nothing for an xlsx file.
// covers: Novis\Spreadsheet\Reader::names, Novis\Spreadsheet\Reader::properties, Novis\Spreadsheet\Reader::macros
#[test]
fn names_properties_and_macros_of_a_workbook() {
    let body = [
        BOOK,
        r#"
var $reader = Reader::open($file);
echo Core\Json::encode($reader->names()), "\n";
var $p = $reader->properties();
echo $p->title ?? "null", " ", $p->author ?? "null", " ", $p->subject ?? "null", " ", $p->created?->toIso() ?? "null", "\n";
echo Core\Arr::count($reader->macros()), "\n";
"#,
    ]
    .concat();
    assert_eq!(
        run("names-properties", None, &body),
        "[{\"name\":\"Rate\",\"reference\":\"Shop!$B$2\",\"sheet\":null}]\nPrices Shop null 2024-01-01T00:00:00Z\n0\n"
    );
}

/// A CSV file has one sheet, `Sheet1`, every value a string, and an empty list from each member
/// that reads a part a CSV file cannot have, while a sheet that does not exist still throws.
// covers: Novis\Spreadsheet\Reader::links, Novis\Spreadsheet\Reader::notes, Novis\Spreadsheet\Reader::tables, Novis\Spreadsheet\Reader::images
#[test]
fn a_csv_file_has_no_parts_but_its_rows() {
    let out = run(
        "csv",
        None,
        r#"
var $reader = Reader::open("Item,Price\nWidget,2.50\n" as bytes);
echo $reader->sheets()[0]->name, "\n";
foreach ($reader->rows(0, {header: true}) as Row $row) {
    echo Core\Json::encode($row->values), "\n";
}
echo Core\Arr::count($reader->links(0)), Core\Arr::count($reader->notes(0)), Core\Arr::count($reader->tables(0)), Core\Arr::count($reader->images(0)), Core\Arr::count($reader->merged(0)), Core\Arr::count($reader->names()), Core\Arr::count($reader->macros()), "\n";
echo $reader->properties()->title ?? "null", "\n";
try {
    $reader->links(1);
} catch (LogicError $e) {
    echo $e->message, "\n";
}
"#,
    );
    assert_eq!(
        out,
        "Sheet1\n{\"Item\":\"Widget\",\"Price\":\"2.50\"}\n0000000\nnull\nA CSV file has one sheet, `Sheet1` at position 0. It has no sheet `1`.\n"
    );
}

/// A workbook opened from a file the program opened reads the same rows as its bytes, and a
/// sheet name or position that does not exist throws a `LogicError` from each member that takes one.
// covers: Novis\Spreadsheet\Reader::open, Novis\Spreadsheet\Reader::links, Novis\Spreadsheet\Reader::notes, Novis\Spreadsheet\Reader::tables, Novis\Spreadsheet\Reader::images, Novis\Spreadsheet\Reader::merged
#[test]
fn a_file_reads_as_its_bytes_and_a_missing_sheet_throws() {
    let body = [
        BOOK,
        r#"
Core\IO::writeStream("book.xlsx", [$file], {});
Core\IO\File $in = Core\IO::open("book.xlsx", Core\IO::FileMode::Read);
array<tainted string> $fromFile = [];
foreach (Reader::open($in)->rows("Shop") as Row $row) {
    $fromFile[] = Core\Json::encode($row->values);
}
$in->close();
array<tainted string> $fromBytes = [];
foreach (Reader::open($file)->rows("Shop") as Row $row) {
    $fromBytes[] = Core\Json::encode($row->values);
}
echo $fromFile == $fromBytes ? "the same rows" : "different rows", "\n";
var $reader = Reader::open($file);
int $thrown = 0;
try { $reader->links("Blog"); } catch (LogicError $e) { $thrown++; }
try { $reader->notes(5); } catch (LogicError $e) { $thrown++; }
try { $reader->tables(-1); } catch (LogicError $e) { $thrown++; }
try { $reader->images("Blog"); } catch (LogicError $e) { $thrown++; }
try { $reader->merged(2); } catch (LogicError $e) { $thrown++; }
echo $thrown, " thrown\n";
"#,
    ]
    .concat();
    assert_eq!(
        run(
            "file",
            Some("[capabilities.fs]\nread = [\".\"]\nwrite = [\".\"]\n"),
            &body
        ),
        "the same rows\n5 thrown\n"
    );
}
