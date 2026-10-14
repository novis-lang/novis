//! `Novis\Spreadsheet\Chart`, run through the built binary: each type writes its own chart
//! element when `Sheet::chart` places it, a chart keeps its options and does not change when
//! `series` or `combine` returns a new one, and only two different types of column, line and area
//! combine, as `rule:core-classes/spreadsheet-sheet-end-parts` places them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("spreadsheet-chart")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `body` after the `use` lines in the case's own directory and returns its stdout, failing
/// the test on a non-zero exit.
fn run(case: &str, body: &str) -> String {
    let dir = scratch(case);
    let program =
        format!("<?nvs\nuse Novis\\Spreadsheet\\Chart;\nuse Novis\\Spreadsheet\\Workbook;\n{body}");
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

/// Each of the seven types, placed on one sheet, writes its own part with its own element: the
/// direction of a column and a bar chart, the hole of a doughnut, and a scatter's x values.
// covers: Novis\Spreadsheet\Chart::constructor, Novis\Spreadsheet\Chart::column, Novis\Spreadsheet\Chart::bar, Novis\Spreadsheet\Chart::line, Novis\Spreadsheet\Chart::area, Novis\Spreadsheet\Chart::pie, Novis\Spreadsheet\Chart::doughnut, Novis\Spreadsheet\Chart::scatter, Novis\Spreadsheet\Chart::series
#[test]
fn each_type_writes_its_own_chart_element() {
    let out = run(
        "types",
        r##"
var $book = Workbook::createInMemory();
var $sheet = $book->sheet("Shop");
$sheet->addAll([["price" => 4.5, "sold" => 120], ["price" => 9.0, "sold" => 75]]);
var $charts = [Chart::column(), Chart::bar(), Chart::line(), Chart::area(), Chart::pie(), Chart::doughnut(), Chart::scatter()];
int $row = 2;
foreach ($charts as var $chart) {
    $sheet->chart("D{$row}", $chart->series($sheet->column("sold"), {categories: $sheet->column("price")}));
    $row = $row + 20;
}
bytes $file = $book->finish()->bytes ?? Core\Bytes::fill(0, 0);
var $marks = ['<c:barDir val="col"/>', '<c:barDir val="bar"/>', "<c:lineChart>", "<c:areaChart>", "<c:pieChart>", '<c:holeSize val="50"/>', "<c:xVal><c:numRef><c:f>Shop!\$A\$2:\$A\$3</c:f>"];
int $n = 1;
foreach ($marks as string $mark) {
    var $xml = Core\Zip::read($file, "xl/charts/chart{$n}.xml") as string;
    echo $n, " ", Core\Str::contains($xml, $mark) ? "yes" : "no", " ", Core\Str::contains($xml, "<c:valAx>") ? "axes" : "none", "\n";
    $n = $n + 1;
}
"##,
    );
    assert_eq!(
        out,
        "1 yes axes\n2 yes axes\n3 yes axes\n4 yes axes\n5 yes none\n6 yes none\n7 yes axes\n"
    );
}

/// The options a type method takes are the chart's fields, `series` and `combine` return a new
/// chart and leave the first one as it was, and the combined chart keeps the first chart's title.
// covers: Novis\Spreadsheet\Chart::constructor, Novis\Spreadsheet\Chart::series, Novis\Spreadsheet\Chart::combine, Novis\Spreadsheet\Chart::area, Novis\Spreadsheet\Chart::doughnut, Novis\Spreadsheet\Chart::scatter
#[test]
fn a_chart_keeps_its_options_and_does_not_change() {
    let out = run(
        "fields",
        r##"
var $area = Chart::area({title: "Orders", xTitle: "Quarter", yTitle: "Count", labels: true});
var $one = $area->series("B2:B5", {categories: "A2:A5", name: "Shop", color: "#2CA02C"});
echo $area->title ?? "-", " ", $area->xTitle ?? "-", " ", $area->yTitle ?? "-", " ", $area->labels ? "labels" : "plain", "\n";
echo Core\Arr::count($area->series), " ", Core\Arr::count($one->series), "\n";
echo $one->series[0]?->values ?? "-", " ", $one->series[0]?->categories ?? "-", " ", $one->series[0]?->name ?? "-", " ", $one->series[0]?->color ?? "-", "\n";
var $doughnut = Chart::doughnut();
var $scatter = Chart::scatter({title: "Points"});
echo $doughnut->title ?? "none", " ", $doughnut->labels ? "labels" : "plain", " ", $scatter->title ?? "none", "\n";
var $both = $one->combine(Chart::line({title: "Costs"})->series("C2:C5"));
echo Core\Arr::count($one->combined), " ", Core\Arr::count($both->combined), " ", $both->title ?? "-", " ", Core\Arr::count($both->series), "\n";
"##,
    );
    assert_eq!(
        out,
        "Orders Quarter Count labels\n0 1\nB2:B5 A2:A5 Shop #2CA02C\nnone plain Points\n0 1 Orders 1\n"
    );
}

/// Every pair of the seven types is combined and placed: only two different types of column,
/// line and area are written, and every other pair throws a `LogicError` that names both types.
// covers: Novis\Spreadsheet\Chart::combine, Novis\Spreadsheet\Chart::column, Novis\Spreadsheet\Chart::bar, Novis\Spreadsheet\Chart::line, Novis\Spreadsheet\Chart::area, Novis\Spreadsheet\Chart::pie, Novis\Spreadsheet\Chart::doughnut, Novis\Spreadsheet\Chart::scatter, Novis\Spreadsheet\Chart::series
#[test]
fn only_two_different_types_of_column_line_and_area_combine() {
    let out = run(
        "combine",
        r##"
var $book = Workbook::createInMemory();
var $sheet = $book->sheet("Sales");
$sheet->addAll([["sales" => 120, "costs" => 80], ["sales" => 150, "costs" => 90]]);
var $types = [Chart::column(), Chart::bar(), Chart::line(), Chart::area(), Chart::pie(), Chart::doughnut(), Chart::scatter()];
foreach ($types as var $first) {
    string $line = "";
    foreach ($types as var $second) {
        try {
            $sheet->chart("D2", $first->series("A2:A3")->combine($second->series("B2:B3")));
            $line = $line . "+";
        } catch (LogicError $e) {
            $line = $line . (Core\Str::contains($e->message, "combines") ? "." : "?");
        }
    }
    echo $line, "\n";
}
"##,
    );
    assert_eq!(
        out,
        "..++...\n.......\n+..+...\n+.+....\n.......\n.......\n.......\n"
    );
}
