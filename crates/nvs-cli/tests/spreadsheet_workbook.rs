//! `Novis\Spreadsheet\Workbook` and `Written`, the Novis-source half of the built-in spreadsheet
//! writer, run through the built binary: each target writes the same file for the same rows,
//! `finish` counts the rows of each sheet, and `define` writes a name, as
//! `rule:core-classes/spreadsheet-writes-as-a-stream` and
//! `rule:core-classes/spreadsheet-sheet-end-parts` place them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("spreadsheet-workbook")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `body` after the `use` lines in the case's directory, with `extra` arguments before the
/// program, and returns its stdout, failing the test on a non-zero exit.
fn run_in(dir: &Path, extra: &[&str], body: &str) -> String {
    let program = format!(
        "<?nvs\nuse Novis\\Spreadsheet\\Workbook;\nuse Novis\\Spreadsheet\\Written;\nuse Novis\\Spreadsheet\\Reader;\n{body}"
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
    String::from_utf8(out.stdout).expect("the output is UTF-8")
}

/// Runs `body` with no `nvs.toml` and no request.
fn run(case: &str, body: &str) -> String {
    run_in(&scratch(case), &[], body)
}

/// A workbook in memory has one row count per sheet, a size equal to its bytes, and bytes that
/// read back as the rows it was given, with the header row from the keys.
// covers: Novis\Spreadsheet\Workbook::createInMemory, Novis\Spreadsheet\Workbook::sheet, Novis\Spreadsheet\Workbook::finish
#[test]
fn a_workbook_in_memory_counts_its_rows_and_reads_back() {
    let out = run(
        "in-memory",
        r#"
var $book = Workbook::createInMemory();
$book->sheet("Orders")->addAll([["order" => 1042, "total" => 19.5], ["order" => 1043, "total" => 5.0]]);
$book->sheet("Notes", {position: 0})->add(["Shipped"]);
var $written = $book->finish();
echo $written->rows["Orders"], " ", $written->rows["Notes"], "\n";
bytes $file = $written->bytes ?? Core\Bytes::fill(0, 0);
echo $written->size == Core\Bytes::length($file) ? "size matches" : "size differs", "\n";
var $reader = Reader::open($file);
foreach ($reader->sheets() as var $sheet) {
    echo $sheet->position, " ", $sheet->name, "\n";
}
foreach ($reader->rows("Orders") as var $row) {
    echo Core\Json::encode($row->values), "\n";
}
"#,
    );
    assert_eq!(
        out,
        "2 1\nsize matches\n0 Notes\n1 Orders\n{\"A\":\"order\",\"B\":\"total\"}\n{\"A\":1042,\"B\":19.5}\n{\"A\":1043,\"B\":5}\n"
    );
}

/// A sheet name used twice in another case, a second CSV sheet, a row on a finished sheet and a
/// second `finish` each throw a `LogicError`.
// covers: Novis\Spreadsheet\Workbook::sheet, Novis\Spreadsheet\Workbook::finish
#[test]
fn a_sheet_or_a_finish_out_of_order_throws_a_logic_error() {
    let out = run(
        "out-of-order",
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
echo Check::attempt(fn (): void => { var $b = Workbook::createInMemory(); $b->sheet("Shop"); $b->sheet("SHOP"); }), "\n";
echo Check::attempt(fn (): void => { var $b = Workbook::createInMemory({format: "csv"}); $b->sheet("Shop"); $b->sheet("Blog"); }), "\n";
echo Check::attempt(fn (): void => { var $b = Workbook::createInMemory(); var $s = $b->sheet("Shop"); $b->sheet("Blog"); $s->add([1]); }), "\n";
echo Check::attempt(fn (): void => { var $b = Workbook::createInMemory(); $b->sheet("Shop"); $b->finish(); $b->finish(); }), "\n";
echo Check::attempt(fn (): void => { Workbook::createInMemory()->finish(); }), "\n";
"#,
    );
    assert_eq!(
        out,
        "LogicError\nLogicError\nLogicError\nLogicError\nLogicError\n"
    );
}

/// `createForFile` and `createForStream` write the bytes `createInMemory` keeps for the same rows,
/// `finish` leaves the open file open, and a file target's `Written` has no bytes.
// covers: Novis\Spreadsheet\Workbook::createForFile, Novis\Spreadsheet\Workbook::createForStream
#[test]
fn a_file_and_an_open_file_get_the_bytes_memory_keeps() {
    let dir = scratch("file-and-stream");
    fs::write(
        dir.join("nvs.toml"),
        "[capabilities.fs]\nread = [\".\"]\nwrite = [\".\"]\n",
    )
    .unwrap();
    let out = run_in(
        &dir,
        &[],
        r#"
class Shop {
    public static function fill(Workbook $book): Written {
        $book->sheet("Orders")->addAll([["order" => 1, "total" => 2.5], ["order" => 2, "total" => 4.0]]);
        return $book->finish();
    }
}
bytes $memory = Shop::fill(Workbook::createInMemory())->bytes ?? Core\Bytes::fill(0, 0);
var $saved = Shop::fill(Workbook::createForFile("saved.xlsx"));
echo $saved->rows["Orders"], " ", $saved->bytes == null ? "no bytes" : "bytes", "\n";
Core\IO\File $open = Core\IO::open("open.xlsx", Core\IO::FileMode::Write);
Shop::fill(Workbook::createForStream($open));
$open->write("");
$open->close();
echo "the open file stayed open\n";
"#,
    );
    assert_eq!(out, "2 no bytes\nthe open file stayed open\n");
    let saved = fs::read(dir.join("saved.xlsx")).unwrap();
    let open = fs::read(dir.join("open.xlsx")).unwrap();
    assert!(saved.starts_with(b"PK"), "an xlsx file is a zip archive");
    assert_eq!(saved, open, "a path and an open file get the same bytes");
}

/// A path that ends in `.csv` writes CSV, and with no `fs.write` grant `createForFile` throws and
/// writes nothing.
// covers: Novis\Spreadsheet\Workbook::createForFile
#[test]
fn a_csv_path_writes_csv_and_no_grant_writes_nothing() {
    let dir = scratch("csv-and-no-grant");
    fs::write(
        dir.join("nvs.toml"),
        "[capabilities.fs]\nwrite = [\"out\"]\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("out")).unwrap();
    let out = run_in(
        &dir,
        &[],
        r#"
var $book = Workbook::createForFile("out/orders.csv");
$book->sheet("Orders")->add(["order" => 1, "customer" => "Example, Ltd"]);
$book->finish();
try {
    Workbook::createForFile("elsewhere.xlsx");
    echo "written\n";
} catch (Throwable $e) {
    echo "refused\n";
}
"#,
    );
    assert_eq!(out, "refused\n");
    assert_eq!(
        fs::read_to_string(dir.join("out/orders.csv")).unwrap(),
        "order,customer\n1,\"Example, Ltd\"\n"
    );
    assert!(
        !dir.join("elsewhere.xlsx").exists(),
        "nothing is written without a grant"
    );
}

/// A download answers a request with the file as its body, and a name ending in `.csv` sends CSV.
// covers: Novis\Spreadsheet\Workbook::createForDownload
#[test]
fn a_download_sends_the_file_as_the_body() {
    let dir = scratch("download");
    fs::write(
        dir.join("request.nvsr"),
        "--METHOD--\nGET\n--PATH--\n/orders.csv\n--HEADERS--\nhost: example.com\n",
    )
    .unwrap();
    let out = run_in(
        &dir,
        &["--request", "request.nvsr"],
        r#"
var $book = Workbook::createForDownload("orders.csv");
$book->sheet("Orders")->addAll([["order" => 1042, "note" => "=1+2"], ["order" => 1043, "note" => "a \"b\""]]);
$book->finish();
"#,
    );
    assert_eq!(out, "order,note\n1042,'=1+2\n1043,\"a \"\"b\"\"\"\n");
}

/// `fromRows` writes one sheet named by `sheet`, and `bytes` returns the whole file.
// covers: Novis\Spreadsheet\Workbook::fromRows
#[test]
fn from_rows_writes_one_named_sheet() {
    let out = run(
        "from-rows",
        r#"
bytes $file = Workbook::fromRows([["item" => "Pen", "price" => 1.5]], {sheet: "Prices"})->bytes();
foreach (Reader::open($file)->sheets() as var $sheet) {
    echo $sheet->name, "\n";
}
echo Workbook::fromRows([["item" => "Pen", "price" => 1.5]], {format: "csv"})->bytes() as string;
"#,
    );
    assert_eq!(out, "Prices\nitem,price\nPen,1.5\n");
}

/// `define` writes the name into `xl/workbook.xml` with its reference escaped, and a name that is
/// a cell address throws a `LogicError`.
// covers: Novis\Spreadsheet\Workbook::define
// covers: Novis\Spreadsheet\Engine::writerDefine
#[test]
fn define_writes_a_defined_name() {
    let out = run(
        "define",
        r#"
var $book = Workbook::createInMemory();
$book->sheet("Shop")->add(["qty" => 1]);
$book->define("Total", "Shop!\$A\$2&");
try {
    $book->define("B2", "Shop!\$A\$2");
} catch (LogicError $e) {
    echo "B2 is refused\n";
}
bytes $file = $book->finish()->bytes ?? Core\Bytes::fill(0, 0);
echo Core\Str::contains(Core\Zip::read($file, "xl/workbook.xml") as string, "<definedName name=\"Total\">Shop!\$A\$2&amp;</definedName>") ? "defined" : "missing", "\n";
"#,
    );
    assert_eq!(out, "B2 is refused\ndefined\n");
}

/// `new Written` keeps the three values it is made from.
// covers: Novis\Spreadsheet\Written::constructor
#[test]
fn a_written_keeps_the_values_it_is_made_from() {
    let out = run(
        "written",
        r#"
var $written = new Written(["Orders" => 3, "Notes" => 1], 120, null);
echo $written->rows["Orders"], " ", $written->rows["Notes"], " ", $written->size, " ", $written->bytes == null ? "null" : "bytes", "\n";
var $kept = new Written(["Orders" => 0], 2, "PK" as bytes);
echo Core\Bytes::length($kept->bytes ?? Core\Bytes::fill(0, 0)), "\n";
"#,
    );
    assert_eq!(out, "3 1 120 null\n2\n");
}
