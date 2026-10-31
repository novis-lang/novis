---
looper: 1
title: "Novis\\Spreadsheet evaluates, writes ods, reads every part of every format, and fills across sheets"
summary: "every Not-shipped part of the spreadsheet rules is built and tested, and no spreadsheet rule fragment has a Not-shipped section left."
context: [docs/rules/core-classes/spreadsheet-roster.md, docs/rules/core-classes/spreadsheet-evaluation-and-roster.md, extensions/spreadsheet/src/lib.rs, wit/spreadsheet.wit, extensions/spreadsheet/manifest.json]
stages:
  - id: 1
    title: "a generator body reads shapes, and every enum but CellError lives in the class that uses it"
    status: done
    context: [crates/nvs-ir/src/lower/generator.rs, crates/nvs-ir/src/lower/mod.rs, extensions/spreadsheet/nvs/Reader.nvs, extensions/spreadsheet/nvs/Style.nvs, docs/rules/enums/an-enum-lives-in-the-class-that-uses-it.md]
  - id: 2
    title: "Cell::rich writes rich text in one cell"
    status: done
    context: [extensions/spreadsheet/nvs/Cell.nvs, extensions/spreadsheet/nvs/Workbook.nvs, extensions/spreadsheet/src/cells.rs, docs/rules/core-classes/spreadsheet-sheet-end-parts.md]
  - id: 3
    title: "a CSV reads and writes in any dialect Core\\Csv takes and in any Core\\Charset"
    status: done
    context: [extensions/spreadsheet/nvs/Reader.nvs, extensions/spreadsheet/nvs/Workbook.nvs, crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/encoding.rs, docs/rules/core-classes/spreadsheet-has-no-io.md]
  - id: 4
    title: "an ods reads every extra, its own number styles and its formula text, and names are in the read's locale"
    status: done
    context: [extensions/spreadsheet/src/read/ods.rs, extensions/spreadsheet/src/read/extras.rs, extensions/spreadsheet/src/format/date.rs, extensions/spreadsheet/src/format/locale.rs, docs/rules/core-classes/spreadsheet-reads-as-a-stream.md]
  - id: 5
    title: "one decoder turns xls and xlsb formula tokens into formula text"
    status: done
    context: [extensions/spreadsheet/src/read/binary.rs, extensions/spreadsheet/src/read/xls.rs, extensions/spreadsheet/src/read/mod.rs, crates/nvs-ext/tests/spreadsheet_read.rs]
  - id: 6
    title: "an xlsb reads its links, notes, names, tables and images"
    status: done
    context: [extensions/spreadsheet/src/read/binary.rs, extensions/spreadsheet/src/read/extras.rs, crates/nvs-ext/tests/spreadsheet_read.rs]
  - id: 7
    title: "an xls reads its links, notes, names, images and properties"
    status: done
    context: [extensions/spreadsheet/src/read/xls.rs, extensions/spreadsheet/src/read/cfb.rs, extensions/spreadsheet/src/read/extras.rs, crates/nvs-ext/tests/spreadsheet_read.rs]
  - id: 8
    title: "a template fills lists on several sheets, from an Iterator too, and moves shared formulas and x14 formats"
    status: done
    context: [extensions/spreadsheet/src/template/mod.rs, extensions/spreadsheet/src/template/shift.rs, extensions/spreadsheet/src/template/sheet.rs, extensions/spreadsheet/nvs/Template.nvs, docs/rules/core-classes/spreadsheet-template-fill.md]
  - id: 9
    title: "the evaluator parses formula text and computes a workbook in dependency order"
    status: done
    context: [extensions/spreadsheet/src/template/shift.rs, extensions/spreadsheet/src/read/mod.rs, docs/rules/core-classes/spreadsheet-value-types.md]
  - id: 10
    title: "the function set, and Reader::evaluate"
    status: open
    context: [extensions/spreadsheet/src/format/mod.rs, extensions/spreadsheet/nvs/Reader.nvs, docs/rules/core-classes/spreadsheet-formula-is-a-value.md]
  - id: 11
    title: "Workbook writes an ods with values, styles, columns and formulas"
    status: open
    context: [extensions/spreadsheet/src/writer.rs, extensions/spreadsheet/src/zip.rs, extensions/spreadsheet/src/styles.rs, extensions/spreadsheet/nvs/Workbook.nvs, docs/rules/core-classes/spreadsheet-writes-as-a-stream.md]
  - id: 12
    title: "an ods carries every sheet-end part but charts"
    status: open
    context: [extensions/spreadsheet/src/tail.rs, extensions/spreadsheet/src/table.rs, extensions/spreadsheet/src/conditional.rs, extensions/spreadsheet/src/drawing.rs, docs/rules/core-classes/spreadsheet-sheet-end-parts.md]
  - id: 13
    title: "an ods carries charts, the component's size is recorded, and no rule owes anything"
    status: open
    context: [extensions/spreadsheet/src/chart.rs, crates/nvs-ext/tests/builtin.rs, tools/nv/cmd/owners.ts]
  - id: 14
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: generator-reads-a-shape-and-enums-are-owned
    stage: 1
    name: "a generator body builds and reads a shape, and the spreadsheet enums are reached through the class that uses them"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/iter/a-generator-builds-and-reads-a-shape.nvst, --case, tests/conformance/novis/spreadsheet-an-enum-lives-in-the-class-that-uses-it.nvst]
    tests: [tests/conformance/iter/a-generator-builds-and-reads-a-shape.nvst, tests/conformance/novis/spreadsheet-an-enum-lives-in-the-class-that-uses-it.nvst]
  - id: generator-lowering-keeps-shape-classes
    stage: 1
    name: "nvs-ir: the advance body of a generator returns the shape classes it records"
    run: [cargo, test, --test, generator_shapes]
    tests: [a_generator_body_that_builds_a_shape_declares_its_class, a_generator_body_that_reads_an_extension_shape_declares_its_class]
  - id: rich-cell
    stage: 2
    name: "Cell::rich writes one cell of styled runs, a run takes font fields only, and the reader reads it back as its text"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-a-rich-cell-writes-its-runs.nvst, --case, tests/conformance/novis/spreadsheet-a-rich-run-takes-font-fields-only.nvst]
    tests: [tests/conformance/novis/spreadsheet-a-rich-cell-writes-its-runs.nvst, tests/conformance/novis/spreadsheet-a-rich-run-takes-font-fields-only.nvst]
  - id: csv-dialect-and-encoding
    stage: 3
    name: "a CSV reads and writes with another separator, quote and charset, and a UTF-16 BOM reads with no option"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-reads-csv-in-another-dialect-and-charset.nvst, --case, tests/conformance/novis/spreadsheet-writes-csv-in-another-dialect-and-charset.nvst, --case, tests/conformance/novis/spreadsheet-a-character-the-charset-lacks-throws-naming-its-cell.nvst]
    tests: [tests/conformance/novis/spreadsheet-reads-csv-in-another-dialect-and-charset.nvst, tests/conformance/novis/spreadsheet-writes-csv-in-another-dialect-and-charset.nvst, tests/conformance/novis/spreadsheet-a-character-the-charset-lacks-throws-naming-its-cell.nvst]
  - id: ods-reads-every-extra
    stage: 4
    name: "nvs-ext: an ods reads its links, notes, names, tables, images and properties, renders text under its own number style, and gives its formula as A1 text"
    run: [cargo, test, --test, spreadsheet_read]
    tests: [ods_links_notes_names_tables_and_images_are_read, ods_properties_are_read_from_its_meta_part, an_ods_cell_text_uses_the_files_own_number_style, an_ods_formula_reads_as_a1_formula_text]
  - id: names-follow-the-locale
    stage: 4
    name: "extensions/spreadsheet: month and day names are rendered in the read's locale, and OpenFormula translates both ways"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, format]
    tests: [month_and_day_names_follow_the_locale, a_locale_with_no_names_row_renders_english_names, openformula_and_a1_text_translate_both_ways]
  - id: formula-tokens-decode
    stage: 5
    name: "nvs-ext: xls and xlsb formula tokens decode to A1 text, shared formulas included, and formulas: true gives a Formula for both"
    run: [cargo, test, --test, spreadsheet_read]
    tests: [every_operand_and_operator_token_decodes_to_a1_text, a_function_token_decodes_by_excels_function_table, an_xls_shared_formula_decodes_relative_to_its_cell, an_xls_and_an_xlsb_formula_read_as_a_formula]
  - id: xlsb-reads-every-extra
    stage: 6
    name: "nvs-ext: an xlsb reads its links, notes, names, tables and images, and follows nothing they name"
    run: [cargo, test, --test, spreadsheet_read]
    tests: [xlsb_links_notes_names_tables_and_images_are_read, an_xlsb_external_link_reads_as_data]
  - id: xls-reads-every-extra
    stage: 7
    name: "nvs-ext: an xls reads its links, notes, names, images and properties, and has no tables"
    run: [cargo, test, --test, spreadsheet_read]
    tests: [xls_links_notes_names_and_images_are_read, xls_properties_are_read_from_its_summary_streams, an_xls_has_no_tables]
  - id: template-second-wave
    stage: 8
    name: "extensions/spreadsheet: lists on two sheets each repeat and shift, a shared formula gets a group per copy, and an x14 format moves"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, template]
    tests: [lists_on_two_sheets_each_repeat_and_shift_every_reference_to_their_sheet, a_shared_formula_in_a_repeated_row_gets_its_own_group_per_copy, an_x14_conditional_format_moves_with_its_rows, a_nul_in_a_filled_value_is_escaped_as_every_written_cell_is, a_flat_key_and_a_nested_path_to_the_same_name_throw, a_date_into_a_cell_with_no_date_format_gets_its_types_format, a_missing_placeholder_throws_at_fill_whatever_the_sheet_holds, the_same_list_named_twice_throws_naming_it, items_for_a_list_the_fill_did_not_return_throw_naming_it]
  - id: table-formula-form
    stage: 8
    name: "extensions/spreadsheet: a calculated column's structured reference is written in the form Excel stores"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, excel]
    tests: [a_calculated_column_is_written_in_the_form_excel_stores]
  - id: template-second-wave-runs
    stage: 8
    name: "a template fills lists on two sheets, and an Iterator in the data is a list"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-a-template-fills-lists-on-two-sheets.nvst, --case, tests/conformance/novis/spreadsheet-a-template-list-is-an-iterator.nvst]
    tests: [tests/conformance/novis/spreadsheet-a-template-fills-lists-on-two-sheets.nvst, tests/conformance/novis/spreadsheet-a-template-list-is-an-iterator.nvst]
  - id: evaluator-core
    stage: 9
    name: "extensions/spreadsheet: formula text parses, operators follow Excel, cells compute in dependency order, and what cannot be computed is listed"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, evaluate]
    tests: [formula_text_parses_into_references_operators_and_calls, operators_follow_excels_precedence_and_coercion, cells_compute_in_dependency_order_across_sheets, defined_names_and_structured_references_resolve, a_circular_reference_is_listed_and_not_computed, an_external_reference_is_listed_and_never_followed, a_function_outside_the_set_is_listed_and_its_cell_keeps_its_stored_result]
  - id: evaluator-functions
    stage: 10
    name: "extensions/spreadsheet: every function of the set has a case, and TODAY, NOW, RAND, INDIRECT and OFFSET are listed"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, evaluate]
    tests: [every_function_of_the_set_has_a_case, a_volatile_or_dynamic_function_is_listed, a_computed_result_reads_by_the_read_table]
  - id: reader-evaluate-runs
    stage: 10
    name: "Reader::evaluate computes a report the writer wrote, and lists what it could not compute"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-evaluate-computes-a-report-it-wrote.nvst, --case, tests/conformance/novis/spreadsheet-evaluate-lists-what-it-could-not-compute.nvst]
    tests: [tests/conformance/novis/spreadsheet-evaluate-computes-a-report-it-wrote.nvst, tests/conformance/novis/spreadsheet-evaluate-lists-what-it-could-not-compute.nvst]
  - id: ods-writer-core
    stage: 11
    name: "extensions/spreadsheet: an ods is a valid package, every value writes as its ods type, a formula is OpenFormula with its cached value, and calamine reads it back"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, ods]
    tests: [an_ods_is_a_package_with_its_mimetype_first_and_stored, every_value_type_writes_as_its_ods_type, a_formula_is_written_as_openformula_with_its_cached_value, equal_style_combinations_share_one_ods_style, columns_header_defaults_and_freeze_are_written_to_ods, two_ods_writes_of_equal_input_are_byte_equal, calamine_reads_back_what_the_ods_writer_wrote]
  - id: ods-writer-runs
    stage: 11
    name: "Workbook writes an ods from the same calls, and the reader reads it back"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-writes-ods-from-the-same-calls.nvst]
    tests: [tests/conformance/novis/spreadsheet-writes-ods-from-the-same-calls.nvst]
  - id: ods-writer-parts
    stage: 12
    name: "extensions/spreadsheet: an ods carries merges, links, notes, validation, hidden parts, groups, tables, conditional formats, images, print setup and protection"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, ods]
    tests: [ods_merges_links_and_notes_are_written_in_their_cells, ods_validation_is_written_before_the_first_sheet, ods_hidden_rows_columns_sheets_and_groups_are_written, ods_tables_are_database_ranges_with_their_filter, ods_conditional_formats_are_written, ods_images_print_setup_and_protection_are_written]
  - id: ods-writer-charts
    stage: 13
    name: "extensions/spreadsheet: every chart type of the subset and the combination are written to an ods"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, ods]
    tests: [every_chart_type_of_the_subset_is_written_to_ods, a_combination_of_two_types_is_written_to_ods]
  - id: component-size-recorded
    stage: 13
    name: "nvs-ext: the spreadsheet component's recorded size matches the bytes the binary carries"
    run: [cargo, test, --test, builtin, the_recorded_growth_matches_the_embedded_components]
    tests: [the_recorded_growth_matches_the_embedded_components]
  - id: no-spreadsheet-rule-owes-anything
    stage: 13
    name: "no spreadsheet rule fragment has a Not shipped or Not on disk section"
    run: [git, grep, -n, -E, "\\*\\*(Not shipped|Not on disk)\\.", --, "docs/rules/core-classes/spreadsheet-*.md"]
    exit: nonzero
    stdout_exact: ""
  - id: owners-closes
    stage: 13
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, spreadsheet-second-wave]
    reach: true
    floor: false
---

# Novis\Spreadsheet evaluates, writes ods, reads every part of every format, and fills across sheets

## Why

The first release ships every part a rule describes. Nine spreadsheet rule fragments still say
**Not shipped.** That covers an evaluate call, writing ods, five extras and the properties from three
read formats, CSV in another dialect or charset, `Cell::rich`, template lists on more than one
sheet, the enums at namespace scope, and a compiler gap that keeps `Reader::rows` split in two.
This goal builds each one. Each fragment then says what is on disk and nothing more.

`release-hygiene` turned each Not-shipped section into a gap record under `data/gaps/` that names
this goal. Each item deletes its own record when it lands, and `bun nv owners --closes
spreadsheet-second-wave` is red until the last one is gone.

## Standing decisions

**The gate on `release-hygiene`.** Stage 1 starts when `bun nv owners --check` exits 0 and
`bun nv owners --closes spreadsheet-second-wave` lists this goal's records. A Not-shipped part with
no record gets one from the session that builds it, in the same commit that deletes it. Home: the
item's commit.

**Scope is the Not-shipped sections.** That covers every `**Not shipped.**` paragraph of
`docs/rules/core-classes/spreadsheet-*.md` and the generator sentence at
`docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:39`. It also covers the bullets of
`extensions/spreadsheet/src/lib.rs:38`'s **Not done yet.** list from the tables bullet to the
template values bullet (`:42` to `:53`). Stage 8 builds them as **Template values** below says.
Each item deletes the bullet it builds. The bullets after `:53` are not named in this goal, and
`release-hygiene`'s records say who owns them. Safe fallback: a bullet that turns out to block an item here is fixed with that
item.

**No new crate.** Every piece is our own Rust in `extensions/spreadsheet/`. That includes the
evaluator, the token decoder, the OpenFormula translator, the ods writer and the month and day name
tables. `calamine` stays a host dev-dependency and reads written ods back as a second opinion.
`ironcalc`, `umya-spreadsheet` and `rust_xlsxwriter` stay out
(`rule:core-classes/spreadsheet-evaluation-and-roster`). Safe fallback: a crate is added only if a
format's own specification cannot be met without it. It must be pure Rust, build for
`wasm32-wasip2`, carry a license `deny.toml` allows, and pass `cargo deny --manifest-path
extensions/spreadsheet/Cargo.toml --target wasm32-wasip2 --locked check`. The roster fragment then
names it.

**The generator fix.** `lower_generator_advance` takes `low.anon_fns` and `low.callables` out of its
`Lowering` and never takes `low.shapes` (`crates/nvs-ir/src/lower/generator.rs:910`). So a shape
class that a generator body records never reaches the unit, and codegen fails with E0901, or the run
fails with "the program declares no class for the shape". The fix takes `low.shapes` and extends
`classes` with it, as an ordinary method does (`crates/nvs-ir/src/lower/mod.rs:1619`). The
`self::` panic in a generator body (`crates/nvs-ir/src/lower/mod.rs:2878`) is a different gap and
stays out. `Reader::walk` then reads each batch itself, and `batch` goes.

**Enums.** Every manifest enum but `CellError` names an `owner`, which `crates/nvs-ext/src/manifest.rs:47`
already supports. `Line`, `Underline`, `Align` and `Valign` go to `Style`. `Visibility` and
`SheetKind` go to `Reader`. `RuleKind`, `Operator`, `TotalFunction`, `ConditionalKind` and
`Orientation` go to `Sheet`. `ChartKind` and `Legend` go to `Chart`, and `ChartKind` is renamed
`Chart::Kind`, because `rule:enums/an-enum-lives-in-the-class-that-uses-it` drops the owner's word
from a name that repeats it. Enums cross the WIT by their cases, so the rename needs no WIT change. `CellError` stays at namespace
scope, because no signature names it (`rule:core-classes/spreadsheet-roster`). The rule's
"`kind` (`'worksheet'` or `'chartsheet'`)" becomes `Reader::SheetKind`, which is what `sheets()`
returns today. Home: `rule:core-classes/spreadsheet-roster` and
`rule:core-classes/spreadsheet-styles-and-defaults`.

**`Cell::rich(array<{text: string, style?: Style}> $runs): Cell`**, the roster's signature. A run's
`Style` sets font fields only: `font`, `size`, `bold`, `italic`, `strike`, `underline` and
`color`. Any other field throws `LogicError` naming the field and the run, because Excel styles a
run's font and nothing else. The cell's fill, border, alignment and format come from its row and
column. An empty list throws `LogicError`. The reader reads a rich cell as its runs' text joined.
`Cell` gets a readonly `runs` beside `value`, and a rich cell has `value` `null`.

**CSV options.** `Reader::open($source, {csv?: {separator?: string, quote?: string, escape?: string,
encoding?: Core\Charset}})`. `Workbook`'s options and `fromRows`'s take `csv?: {separator?: string,
quote?: string, encoding?: Core\Charset}`. Each field means what `Core\Csv`'s option of that name
means, and `Core\Csv` checks it. The options are a `csv` field so that a reader knows they apply to
CSV alone. A workbook source reads as the workbook, whatever `csv` says. A source with a UTF-16 BOM
reads as UTF-16 with no option. Without `encoding`, a source that is not UTF-8 goes to the engine
as today. Nothing guesses a charset. A non-UTF-8 source is decoded whole with
`Core\Encoding::decodeText` and parsed with `Core\Csv::parse`. A `Core\IO\File` source is read whole
first. Memory: the decoded text and its records, per request. A write encodes each batch with
`Core\Encoding::encodeText`. A character the charset lacks throws `LogicError` naming the row and
the column. Home: `rule:core-classes/spreadsheet-has-no-io`.

**A file read whole goes through the engine.** `Core\IO\File::read` returns text only, so Novis
cannot read a non-UTF-8 file's bytes. The engine export `sourceBytes(source, offset, ?length)`
returns a range of the lent source through the host's `range`, as every reader call reads it.
`Reader::open` reads a file's first four bytes and a file in another charset through it. A new
`Core\IO\File` byte read was declined, because it is `Core` surface for one reader. The error names
the cell (`B3`), and no charset name, because an enum case has no name at run time. Home:
`rule:core-classes/spreadsheet-has-no-io`.

**Reading extras from every format.** Each extra returns the shape it returns for xlsx.
- **ods:** links are `text:a` in a cell, notes are `office:annotation`, names are
  `table:named-expressions`, tables are `table:database-ranges`, images are `draw:frame` with
  `draw:image` from the package, and properties come from `meta.xml`. An ods cell's `text` renders
  under the file's own `number:*-style`, translated to a format code that the renderer reads
  (`extensions/spreadsheet/src/format/mod.rs:401`).
- **xlsb:** hyperlinks are `BrtHLink` with the sheet's relationships, notes come from the comments
  part, and names are `BrtName` with their tokens decoded. Tables come from the table parts, and
  images from the drawing part, which is XML in an xlsb too.
- **xls:** links are `HLINK`, notes are `NOTE` with its `OBJ` and `TXO`, names are `NAME` with its
  tokens decoded, and images come from the `MSODRAWINGGROUP` picture store with each `OBJ`'s
  anchor. Properties come from the `\x05SummaryInformation` and `\x05DocumentSummaryInformation`
  streams. **An xls has no tables:** BIFF8 has no table part, so `tables` returns `[]`.
Home: `rule:core-classes/spreadsheet-reads-as-a-stream`.

**Month and day names.** One table, `extensions/spreadsheet/src/format/names.rs`, has CLDR's wide
and abbreviated Gregorian month and day names, in format context. It has a row for English and for
each language of the separators table (`extensions/spreadsheet/src/format/locale.rs:46`). The row is
found the way `of_tag` finds separators: by region, then by language, then English. A `[$-407]`
prefix picks its language's names too. The names are copied from the CLDR release that the
`icu_datetime` version in `extensions/intl/Cargo.toml` bakes in, and the module doc names that
release. Memory: static data in the component, about 20 KiB.

**One token decoder for xls and xlsb.** `extensions/spreadsheet/src/read/tokens.rs` turns BIFF8 and
BIFF12 parsed formulas into A1 formula text. The two share their token ids. It decodes every operand
and operator token, `ptgFunc` and `ptgFuncVar` by Excel's built-in function table (the `Ftab` list
of [MS-XLS] 2.5.198.17, one static array), and a shared formula (`ptgExp` and `SHRFMLA`) relative
to its own cell. Names use it, and so do `formulas: true` and `evaluate`. With it, `formulas: true`
gives a `Formula` for an xls and an xlsb cell. An ods cell's OpenFormula is translated to A1 text by
`extensions/spreadsheet/src/openformula.rs`, so an ods gives a `Formula` too. The sentences at
`docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:30` and `:105` change to say so. A token
the decoder does not know makes that cell's formula text `null`. The cell then gives no `Formula`,
and `evaluate` lists it.

**Template fill across sheets.** Each sheet with a block streams as the one sheet does now, in
workbook order. The Novis half sends each list's items in the order its block stands: sheet order,
then row. A block's shift moves every reference to its own sheet, from every sheet and every part,
which the shifter already does for a sheet-qualified reference
(`extensions/spreadsheet/src/template/shift.rs:116`). A sheet that a formula above a block keeps
whole is kept as now. A shared formula in a repeated row gets a new `si` for each copy, and its
`ref` moves with that copy. Safe fallback: each copy writes the formula's full text. An `x14`
conditional format under `extLst` has its `xm:sqref` and `xm:f` moved. An `Iterator` in `$data` is
a list, drained in batches of 1,000 items as an array is sent. Home:
`rule:core-classes/spreadsheet-template-fill`.

**Template values**, the module doc's bullet at `extensions/spreadsheet/src/lib.rs:48`:
- A filled value's text goes through `crate::xml`'s escape, as every written cell's does. So a NUL
  is `_x0000_` (`extensions/spreadsheet/src/xml.rs:5`).
- A `$data` with a flat key `'customer.name'` and a nested `customer` → `name` throws `LogicError`
  naming both. A path never picks one of them in silence.
- **An address with no sheet name is the first sheet.** This is a design choice, and it stays. The
  fragment states it as a fact, beside `set('Summary!B2', …)`.
- A date, time or duration filled into a cell whose style has no date or time format gets the
  write table's format for its type. The template's `styles.xml` gains one `cellXfs` entry for
  each cell style and format pair, so the value reads as a date in Excel and in our reader.
- Every placeholder `$data` lacks throws at `fill`, before any byte is written, whether or not the
  sheet has a list row.
- `templateFill` with one list name twice throws naming that list. `templateItems` with a list the
  fill did not return throws naming it.

**The table formula form.** The tables bullet (`extensions/spreadsheet/src/lib.rs:42`) is a design
choice. Excel itself stores `[@x]` as `Table[[#This Row],[x]]` in the file, so the writer writes
the long form (`extensions/spreadsheet/src/table.rs:411`). The item checks it against a
`rust_xlsxwriter` fixture with a calculated column, and the module doc says it as a fact. Safe
fallback: if the fixture stores the short form, the writer writes the fixture's form.

**The evaluate call.** `Reader::evaluate(): array<{cell: tainted string, kind: string, text: tainted
string}>`.
- **What it computes.** Every formula cell of every sheet, once, in the guest. A circular
  reference is not computed. The call returns what it could not compute: `cell` as `Sheet!A1`, and
  `kind` as `'function'`, `'reference'`, `'circular'` or `'token'`. `text` is the function's name,
  the reference as the file wrote it, or the formula.
- **What changes after it.** Each `rows()` reads a formula cell as its computed result, through the
  read table and the cell's number format. `formulas: true` gives the `Formula` with the computed
  result as its `cached`. A cell it could not compute keeps its stored result.
- **What it never does.** It never runs on its own. Reading still evaluates nothing until the
  program calls it, and the rule's Declined bullet stands.
- **Security.** It does no I/O and follows nothing. An external workbook reference, `INDIRECT`
  and `OFFSET` are listed as references or functions. It runs under the request's CPU and memory
  caps.
- **Memory.** One value per non-empty cell of every sheet, the parsed formulas and the dependency
  order, per request, freed when the reader is. The values live in a sparse grid, so `A:A` walks
  only stored cells.
- **Latency.** One guest call. The work grows with the cells and the references.
- **Formats.** Every format the reader reads, through the token decoder and the OpenFormula
  translator. A CSV has no formulas and returns `[]`.
- **Excel's behaviour.** Numbers are `f64`, and coercion and error values follow Excel. Defined
  names and structured references (`Table[Column]`, `[#This Row]`, `[#Data]`, `[#All]`,
  `[#Headers]` and `[#Totals]`) resolve. The union and intersection operators, array formulas and
  spills are listed.
Home: `rule:core-classes/spreadsheet-evaluation-and-roster`, with the function set below written out
there.

**The function set.** It is one table, `extensions/spreadsheet/src/eval/functions.rs`, and
`every_function_of_the_set_has_a_case` reads that table. These functions are in it:
- Math and statistics: `ABS`, `AVERAGE`, `AVERAGEIF`, `AVERAGEIFS`, `CEILING`, `CEILING.MATH`,
  `COUNT`, `COUNTA`, `COUNTBLANK`, `COUNTIF`, `COUNTIFS`, `EXP`, `FLOOR`, `FLOOR.MATH`, `INT`, `LN`,
  `LOG`, `LOG10`, `MAX`, `MAXIFS`, `MEDIAN`, `MIN`, `MINIFS`, `MOD`, `PI`, `POWER`, `PRODUCT`, `ROUND`,
  `ROUNDDOWN`, `ROUNDUP`, `SIGN`, `SQRT`, `SUBTOTAL` (codes 1 to 11 and 101 to 111), `SUM`, `SUMIF`,
  `SUMIFS`, `SUMPRODUCT`, `TRUNC`.
- Logic: `AND`, `FALSE`, `IF`, `IFERROR`, `IFNA`, `IFS`, `NOT`, `OR`, `SWITCH`, `TRUE`, `XOR`.
- Information: `ISBLANK`, `ISERR`, `ISERROR`, `ISEVEN`, `ISLOGICAL`, `ISNA`, `ISNUMBER`, `ISODD`,
  `ISTEXT`, `NA`.
- Text: `CHAR`, `CLEAN`, `CODE`, `CONCAT`, `CONCATENATE`, `EXACT`, `FIND`, `LEFT`, `LEN`, `LOWER`,
  `MID`, `PROPER`, `REPLACE`, `REPT`, `RIGHT`, `SEARCH`, `SUBSTITUTE`, `TEXT` (through the
  number-format renderer), `TEXTJOIN`, `TRIM`, `UPPER`, `VALUE`.
- Lookup: `CHOOSE`, `COLUMN`, `COLUMNS`, `HLOOKUP`, `INDEX`, `MATCH`, `ROW`, `ROWS`, `VLOOKUP`,
  `XLOOKUP`.
- Date and time: `DATE`, `DATEDIF`, `DAY`, `DAYS`, `EDATE`, `EOMONTH`, `HOUR`, `MINUTE`, `MONTH`,
  `SECOND`, `TIME`, `WEEKDAY`, `YEAR`.

`TODAY`, `NOW`, `RAND` and `RANDBETWEEN` are out, because the result would depend on when or how
often the program reads. `INDIRECT` and `OFFSET` are out, because they make a reference from data.
Every function outside the table is listed too. Safe fallback: a function in the set that cannot
match Excel on a case is removed from the table and the rule, and is listed. It is never computed
differently. Declined, written into the rule: an option to compute `TODAY` and `NOW` from a given
instant (a program that needs one fills a cell).

**Writing ods.** A name ending `.ods`, or `{format: 'ods'}`, writes ods from the same calls
(`extensions/spreadsheet/nvs/Workbook.nvs:253`).
- **The writer holds the workbook.** It is our own, in `extensions/spreadsheet/src/ods/`, and it
  holds the workbook in the guest until `finish()`. ods puts data validation before the first
  sheet, and a merge, a note, a link and an image inside the cells they cover. So a stream that has
  already sent a sheet's rows cannot add them.
- **Cost.** Memory: every cell of the workbook, compact, per request, under the request's cap and
  freed at `finish()`. Latency: no byte of an ods leaves before `finish()`. A download's headers
  still leave at create. xlsx keeps streaming.
- **The parts.** `mimetype` comes first and stored, then `content.xml`, `styles.xml`, `meta.xml`,
  `settings.xml` (freeze, zoom, gridlines and tab colour) and `META-INF/manifest.xml`.
- **Styles.** Every style is an automatic style, written once for each distinct combination.
- **Formulas.** A formula is OpenFormula (`of:=`), translated from the program's A1 text by
  `extensions/spreadsheet/src/openformula.rs`, with its cached value when one was given.
- **The sheet-end parts.** Stage 12 writes every one but charts: tables as `table:database-ranges`
  with their filter, conditional formats as LibreOffice's `calcext:conditional-formats`, outline
  groups as `table:table-row-group`, and the rest in ODF 1.3's own elements. Stage 13 writes charts
  as embedded chart objects, `Object N/content.xml` with `chart:chart`, for the same subset.
  Table styles and totals-row styling have no ods form. They are left out, and the rule says so.
- **Two writes are byte-equal.** The fixed dates and order follow `writer.rs`'s.
Home: `rule:core-classes/spreadsheet-evaluation-and-roster` and
`rule:core-classes/spreadsheet-writes-as-a-stream`, which says "streaming is the only way to write
xlsx", with ods's held workbook and its cost. Declined, written into the rule: streaming ods with
the parts that cannot follow their rows refused (a part that works in one format and throws in the
other is two APIs). Safe fallback: if a LibreOffice element cannot be written byte-equal or read
back, that part throws `LogicError` naming ods, and the rule lists it as a bound.

**Fixtures.** Reader tests build their xlsb, xls and ods bytes in the test, as
`crates/nvs-ext/tests/spreadsheet_read.rs:1314` and `:1797` do. Where a record is too long to build
by hand, the fixture is a file from calamine's MIT test corpus under
`extensions/spreadsheet/tests/fixtures/calamine/`, with its license beside it, as `excel/` holds
`rust_xlsxwriter`'s. Writer tests read back with our reader and with `calamine`.

**Feature proofs.** These members are new and owe the full set (`rule:testing/feature-proofs`):
`Novis\Spreadsheet\Cell::rich` and `Novis\Spreadsheet\Reader::evaluate`. The full set is
`about.md`, three examples, tests, one bench under `benches/members/novis/` and one attack under
`tests/hostile/novis/`. The other items change members that already have their proofs. Those owe a
new example and an `about.md` that says what changed: `Reader::open` (CSV options),
`Workbook::createForFile` and `Workbook::fromRows` (ods and CSV options), `Template::fill` (sheets
and iterators), and each `Reader` extra (the formats). An enum's type page moves with its owner,
for example `docs/examples/types/Novis-Spreadsheet-Align/` to
`docs/examples/types/Novis-Spreadsheet-Style-Align/`, as `Novis-Image-Image-Align` is named.
`bun nv proofs --id '<member>'` owes nothing at the end of each item, and
`bun nv proofs --comments` passes on each new `.nvs`.

**Text an end user reads.** Every new help line, `about.md`, example comment and error message
follows AGENTS.md § *Text an end user reads*.

**The size.** The component grows with the evaluator, the decoder, the names and the ods writer.
Stage 13 runs `the_recorded_growth_matches_the_embedded_components` once, by name. It writes the
size the failure names into `extensions/spreadsheet/src/lib.rs:33` and states the growth in its
commit.

**Rules.** No new rule is made. Each item rewrites the fragment it builds, drops that fragment's
Not-shipped sentence, and adds its tests to the record's `guardedBy`. The session then runs
`bun nv rules --render`, and `bun nv render --website` where a page changed. The website's enum list
is `website/src/content/docs/syntax/values/enums.mdx:3`.

**The evaluator's own bounds**, decided in stage 9 (`extensions/spreadsheet/src/eval/`):
- Formula text the parser cannot read is listed as `token`, with the formula, as an undecoded
  token is. So is text past the stack bounds: parentheses and calls more than 64 deep (Excel's own
  limit), a tree more than 512 nodes deep, or more than 4,096 operators and calls.
- A result of more than one cell, and a range used as one value (`=A1:A3*2`), is a spill or an
  array formula, and is listed as `reference` with the range. `ROW` and `COLUMN` of a reference
  more than one row or column wide are listed as `function`. A call with an argument count Excel
  does not accept is listed as `function`.
- Text converts to a number when it is a number, a percentage, an ISO date or a clock time. Other
  text is `#VALUE!`. A date in a locale's own order (`1/15/2024`) is other text, because the
  result would depend on the machine. The fragment states this bound in stage 10.
- An unknown name is `#NAME?`, as Excel gives. A name chain more than 64 deep is listed as
  `circular`. A sheet-qualified name (`Summary!Rate`) reads only that sheet's names.
- A cell that reads a cell not computed reads its stored value, and is computed.
- `ROW`, `COLUMN`, `ROWS` and `COLUMNS` read no cells, so a reference in them is not a dependency
  and `=ROWS(A1:A9)` in `A1` is not circular.
- `IF` evaluates only the branch it takes, so a function outside the set in the other branch is
  not listed.
Home: the module docs of `eval/mod.rs`, `eval/parse.rs` and `eval/value.rs` now, and the fragment
from stage 10.

**Trade-offs, as a whole.** Security: the new readers parse hostile bytes in the sandbox, the
evaluator computes only on request, and nothing is fetched. Latency: none on any existing path. An
evaluate call and an ods write cost what the rows cost. Memory: a decoded CSV, an evaluated
workbook and an ods being written are each held per request, under the cap. Simplicity: one API
for every format, and a list of what was not computed in place of a silent wrong value.

**Lanes.** Stage 1 runs first, because stages 2 and 3 edit the `.nvs` files whose enums it moves.
Stages 2 and 3 share `Workbook.nvs` and `Reader.nvs` and run in order. Stages 4 to 7 share
`read/extras.rs` and run in order. Stage 8 touches only `template/` and `Template.nvs`, and stage 9
touches only the new `eval/`. Each may run as a side goal beside stages 4 to 7. Stage 10 needs
stages 4, 5 and 9. Stages 11 to 13 need stage 2 (rich cells) and stage 4 (`openformula.rs`), and run
in order. Every stage that adds an export edits `lib.rs`, `wit/spreadsheet.wit` and
`manifest.json`, so two such stages never run at once.

## Stages

### 1. a generator body reads shapes, and every enum but CellError lives in the class that uses it

**Does:** a generator compiles with shapes in its body, `Reader::rows` is one generator, and each
spreadsheet enum is reached through its class.

**Split:** the thirteen enum type pages move as one batch:
`docs/examples/types/Novis-Spreadsheet-{Align,ChartKind,ConditionalKind,Legend,Line,Operator,Orientation,RuleKind,SheetKind,TotalFunction,Underline,Valign,Visibility}/`.
A writing subagent moves them and rewrites their examples once the manifest's owners have landed.
It builds nothing.

- [x] Gate: `bun nv owners --check` exits 0, and `bun nv owners --closes spreadsheet-second-wave` lists this goal's records. `tools/nv/cmd/owners.ts:1`
- [x] `rule:iteration/generators`: the advance body takes `low.shapes` and extends `classes`, as an ordinary method does. `crates/nvs-ir/src/lower/generator.rs:910`, `crates/nvs-ir/src/lower/mod.rs:1619`
- [x] The tests the `generator-lowering-keeps-shape-classes` check names. `crates/nvs-ir/tests/generator_shapes.rs:1`. The conformance case: `tests/conformance/iter/a-generator-builds-and-reads-a-shape.nvst:1`
- [x] `rule:core-classes/spreadsheet-reads-as-a-stream`: `walk` reads each batch itself, `batch` goes, and the sentence at `:39` says the generator reads the batch. `extensions/spreadsheet/nvs/Reader.nvs:189`, `extensions/spreadsheet/nvs/Reader.nvs:241`, `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:39`
- [x] The playbook bullet drops its shape half, and keeps the `self::` half and its `until`. `docs/agent/playbook/writing-novis/a-generator-body-that-names-self.md:1`
- [x] `rule:core-classes/spreadsheet-styles-and-defaults`, `rule:core-classes/spreadsheet-roster`: each enum's `owner` as the standing decision lists. Every `.nvs` that names one uses `Owner::Enum`. `extensions/spreadsheet/manifest.json:259`, `extensions/spreadsheet/nvs/Style.nvs:12`, `extensions/spreadsheet/nvs/Reader.nvs:13`, `extensions/spreadsheet/nvs/Sheet.nvs:23`, `extensions/spreadsheet/nvs/Chart.nvs:45`, `extensions/spreadsheet/nvs/Workbook.nvs:20`
- [x] The conformance case `tests/conformance/novis/spreadsheet-an-enum-lives-in-the-class-that-uses-it.nvst:1`. The enum type pages move, and the website's enum list names the owned enums. `website/src/content/docs/syntax/values/enums.mdx:3`
- [x] The roster's opening sentence, `:24` and its **On disk.** paragraph, the `kind` sentence at `reads-as-a-stream.md:73`, and the styles fragment's Not-shipped line are rewritten. Delete the record. `docs/rules/core-classes/spreadsheet-roster.md:24`, `docs/rules/core-classes/spreadsheet-styles-and-defaults.md:67`

### 2. Cell::rich writes rich text in one cell

**Does:** a program writes styled runs in one cell, and the writer's existing rich-text path is
reached from Novis.

- [x] `rule:core-classes/spreadsheet-roster`: `Cell::rich`, the `runs` property, and the run checks the standing decision gives. `extensions/spreadsheet/nvs/Cell.nvs:4`
- [x] The Novis half sends a rich cell as `cell-value::rich`, each run's style an id from `writer-style`. `extensions/spreadsheet/nvs/Workbook.nvs:349`, `wit/spreadsheet.wit:73`, `extensions/spreadsheet/src/cells.rs:285`
- [x] The CSV path writes a rich cell as its joined text. `extensions/spreadsheet/nvs/Workbook.nvs:588`
- [x] The conformance cases the `rich-cell` check names. `tests/conformance/novis/spreadsheet-a-rich-cell-writes-its-runs.nvst:1`, `tests/conformance/novis/spreadsheet-a-rich-run-takes-font-fields-only.nvst:1`
- [x] Feature proofs for `Novis\Spreadsheet\Cell::rich`: `about.md`, three examples, the bench and the attack. `docs/examples/novis/Spreadsheet-Cell/rich/about.md:1`, `benches/members/novis/Spreadsheet-Cell/rich.nvs:1`, `tests/hostile/novis/Spreadsheet-Cell/rich/01-runs-that-are-huge-or-empty.nvs:1`
- [x] `rule:core-classes/spreadsheet-sheet-end-parts` and the roster drop `Cell::rich` from their Not-shipped and On-disk lines. Delete the record. `docs/rules/core-classes/spreadsheet-roster.md:50`, `docs/rules/core-classes/spreadsheet-sheet-end-parts.md:83`

### 3. a CSV reads and writes in any dialect Core\Csv takes and in any Core\Charset

**Does:** `Reader::open` and `Workbook` take the `csv` options the standing decision gives.

- [x] `rule:core-classes/spreadsheet-has-no-io`: `Reader::open`'s options, the UTF-16 BOM, the whole-text decode, and the dialect passed to `Core\Csv::parse` and `Core\Csv::rows`. `extensions/spreadsheet/nvs/Reader.nvs:34`, `extensions/spreadsheet/nvs/Reader.nvs:291`, `extensions/spreadsheet/nvs/Reader.nvs:377`, `extensions/spreadsheet/nvs/Reader.nvs:396`
- [x] `Workbook`'s and `fromRows`'s `csv` options, the dialect passed to `Core\Csv::format`, and the encode with its error. `extensions/spreadsheet/nvs/Workbook.nvs:12`, `extensions/spreadsheet/nvs/Workbook.nvs:15`, `extensions/spreadsheet/nvs/Workbook.nvs:599`
- [x] The conformance cases the `csv-dialect-and-encoding` check names. `tests/conformance/novis/spreadsheet-reads-csv-in-another-dialect-and-charset.nvst:1`, `tests/conformance/novis/spreadsheet-writes-csv-in-another-dialect-and-charset.nvst:1`, `tests/conformance/novis/spreadsheet-a-character-the-charset-lacks-throws-naming-its-cell.nvst:1`
- [x] A fourth example and an `about.md` paragraph for `Reader::open`, `Workbook::createForFile` and `Workbook::fromRows`. `docs/examples/novis/Spreadsheet-Reader/open/about.md:4`, `docs/examples/novis/Spreadsheet-Workbook/createForFile/about.md:1`, `docs/examples/novis/Spreadsheet-Workbook/fromRows/about.md:1`
- [x] The fragment states the options, the cost and the charset rule, and drops its Not-shipped line. Delete the record. `docs/rules/core-classes/spreadsheet-has-no-io.md:74`

### 4. an ods reads every extra, its own number styles and its formula text, and names are in the read's locale

**Does:** every extra and `properties` read from an ods, an ods cell's `text` uses the file's
number style, and month and day names follow `locale`.

- [x] `rule:core-classes/spreadsheet-reads-as-a-stream`: `ods::read` keeps each cell's links, annotation and image frames, and each sheet's named expressions and database ranges. `extensions/spreadsheet/src/read/ods.rs:116`, `extensions/spreadsheet/src/read/ods.rs:183`
- [x] The extras answer an ods from what `ods::read` kept, and `properties` reads `meta.xml`. The format check stops naming ods. `extensions/spreadsheet/src/read/extras.rs:149`, `extensions/spreadsheet/src/read/extras.rs:475`
- [x] `number:*-style` to a format code, and each cell's `text` rendered under it. `extensions/spreadsheet/src/read/ods.rs:116`, `extensions/spreadsheet/src/format/mod.rs:401`
- [x] `openformula.rs`: OpenFormula to A1 text and back. An ods formula reads as a `Formula` with `formulas: true`. `extensions/spreadsheet/src/openformula.rs:1`, `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:30`
- [x] `names.rs` and the date renderer reading it by locale and by `[$-…]` prefix. `extensions/spreadsheet/src/format/names.rs:1`, `extensions/spreadsheet/src/format/date.rs:18`, `extensions/spreadsheet/src/format/locale.rs:82`
- [x] The tests the `ods-reads-every-extra` and `names-follow-the-locale` checks name. `crates/nvs-ext/tests/spreadsheet_read.rs:1797`, `extensions/spreadsheet/tests/format.rs:1`
- [x] The `about.md` of each extra and of `properties` names ods. The fragment's Not-shipped paragraph loses ods, the number style and the locale sentence. The module doc's ods bullets go. Delete the records. `docs/examples/novis/Spreadsheet-Reader/properties/about.md:8`, `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:113`, `extensions/spreadsheet/src/lib.rs:55`

### 5. one decoder turns xls and xlsb formula tokens into formula text

**Does:** `read/tokens.rs` decodes BIFF8 and BIFF12 parsed formulas, and an xls or xlsb formula cell
gives a `Formula` with `formulas: true`.

- [x] `rule:core-classes/spreadsheet-reads-as-a-stream`: `tokens.rs` with every operand and operator token, the `Ftab` function table, and the shared-formula base. `extensions/spreadsheet/src/read/tokens.rs:1`
- [x] The xlsb cell reader keeps each formula's tokens, and `formulas: true` decodes them. `extensions/spreadsheet/src/read/binary.rs:345`
- [x] The xls reader keeps each `FORMULA`'s tokens and each `SHRFMLA`, and `formulas: true` decodes them. `extensions/spreadsheet/src/read/xls.rs:211`, `extensions/spreadsheet/src/read/xls.rs:329`
- [x] The tests the `formula-tokens-decode` check names. `crates/nvs-ext/tests/spreadsheet_read.rs:1314`
- [x] The sentences at `:30` and `:105` say a formula reads as text from every format. `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:105`

### 6. an xlsb reads its links, notes, names, tables and images

**Does:** every extra reads from an xlsb, through the same shapes as an xlsx.

- [x] `rule:core-classes/spreadsheet-reads-as-a-stream`: `BrtHLink` from the sheet part, scanned to its end as `merged` is, with the sheet's relationships. `extensions/spreadsheet/src/read/binary.rs:293`, `extensions/spreadsheet/src/read/extras.rs:194`
- [x] Notes from the comments part, and names from `BrtName` with `tokens.rs`. `extensions/spreadsheet/src/read/binary.rs:208`, `extensions/spreadsheet/src/read/extras.rs:229`, `extensions/spreadsheet/src/read/extras.rs:299`
- [x] Tables from the table parts, and images from the drawing part. `extensions/spreadsheet/src/read/extras.rs:341`, `extensions/spreadsheet/src/read/extras.rs:379`
- [x] The tests the `xlsb-reads-every-extra` check names. `crates/nvs-ext/tests/spreadsheet_read.rs:1370`
- [x] Each extra's `about.md` names xlsb. The fragment and the module doc lose xlsb. The record also covers xls, so stage 7 deletes it. `docs/examples/novis/Spreadsheet-Reader/links/about.md:1`, `extensions/spreadsheet/src/lib.rs:57`

### 7. an xls reads its links, notes, names, images and properties

**Does:** every extra and `properties` read from an xls, and `tables` returns `[]`.

- [x] `rule:core-classes/spreadsheet-reads-as-a-stream`: `xls::read` keeps `HLINK`, `NOTE` with its `OBJ` and `TXO`, `NAME` and the picture store with each anchor. `extensions/spreadsheet/src/read/xls.rs:231`
- [x] `properties` reads the two summary streams from the compound file. `extensions/spreadsheet/src/read/cfb.rs:77`, `extensions/spreadsheet/src/read/extras.rs:475`
- [x] The extras answer an xls, and `format_name` goes once no format is refused. `extensions/spreadsheet/src/read/extras.rs:149`, `extensions/spreadsheet/src/read/extras.rs:160`
- [x] The decoder gives a reference into another workbook as `[n]Sheet!A1`, as an xlsx file writes it: an xls `SUPBOOK` lists its sheet names, and an xlsb `BrtSupBookSrc` names an external link part whose `BrtSupTabs` lists them. An xlsb name over another workbook then has its reference, and the fragment's sentence on it goes. `extensions/spreadsheet/src/read/tokens.rs:738`, `extensions/spreadsheet/src/read/tokens.rs:877`
- [x] The tests the `xls-reads-every-extra` check names. `crates/nvs-ext/tests/spreadsheet_read.rs:1846`
- [x] Each extra's `about.md` and `properties`' `about.md` stop naming a format that throws. The fragment's Not-shipped paragraph goes, the extras sentence names `tables` on an xls, and the module doc's extras bullet goes. Delete the records. `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:113`, `extensions/spreadsheet/src/lib.rs:57`
  - [x] Every `about.md`, `Reader.nvs`'s `properties` help, the fragment's xls paragraph and `extras.rs`'s module doc say an xls reads every extra.
  - [x] The Not-shipped paragraph, the module doc's extras bullet's first sentence and the gap record go with the decoder item.

### 8. a template fills lists on several sheets, from an Iterator too, and moves shared formulas and x14 formats

**Does:** `fill` takes lists on any sheet, and the two copy faults the fragment names are fixed.

- [x] `rule:core-classes/spreadsheet-template-fill`: the one-sheet check goes, and the sheets with blocks stream in workbook order, each with its own shifts. `extensions/spreadsheet/src/template/mod.rs:646`, `extensions/spreadsheet/src/template/mod.rs:720`, `extensions/spreadsheet/src/template/mod.rs:795`
- [x] A shared formula's `si` and `ref` per copy. `extensions/spreadsheet/src/template/mod.rs:265`, `extensions/spreadsheet/src/template/sheet.rs:95`
- [x] `extLst`'s `x14:conditionalFormatting` moves its `xm:sqref` and `xm:f`. `extensions/spreadsheet/src/template/mod.rs:332`, `extensions/spreadsheet/src/template/shift.rs:265`
- [x] The Novis half sends lists in block order, and an `Iterator` in `$data` is a list drained in batches. `extensions/spreadsheet/nvs/Template.nvs:57`, `extensions/spreadsheet/nvs/Template.nvs:78`, `extensions/spreadsheet/nvs/Template.nvs:141`
- [x] Template values: the escape, the key collision, the date format, the missing placeholder at `fill`, and the two export errors. The first-sheet rule goes into the fragment as a fact. `extensions/spreadsheet/src/template/mod.rs:552`, `extensions/spreadsheet/src/template/mod.rs:720`, `extensions/spreadsheet/src/template/mod.rs:758`, `extensions/spreadsheet/src/lib.rs:1398`, `extensions/spreadsheet/src/lib.rs:1412`, `extensions/spreadsheet/nvs/Template.nvs:66`
- [x] The table formula form: a `rust_xlsxwriter` fixture with a calculated column under `tests/fixtures/excel/`, and the test the `table-formula-form` check names. This item shares no file with the rest of the stage. `extensions/spreadsheet/src/table.rs:411`, `extensions/spreadsheet/tests/excel.rs:1`
- [x] The tests and cases the `template-second-wave` checks name. `extensions/spreadsheet/tests/template.rs:1`, `tests/conformance/novis/spreadsheet-a-template-fills-lists-on-two-sheets.nvst:1`, `tests/conformance/novis/spreadsheet-a-template-list-is-an-iterator.nvst:1`
- [x] A fourth `Template::fill` example, with lists on two sheets. The fragment's Not-shipped paragraph and the module doc's template bullet go. Delete the record. `docs/examples/novis/Spreadsheet-Template/fill/about.md:1`, `docs/rules/core-classes/spreadsheet-template-fill.md:47`, `extensions/spreadsheet/src/lib.rs:42`, `extensions/spreadsheet/src/lib.rs:44`, `extensions/spreadsheet/src/lib.rs:48`

### 9. the evaluator parses formula text and computes a workbook in dependency order

**Does:** `src/eval/` parses A1 formula text, holds a sparse grid of a workbook, and computes every
formula cell with the operators and a first few functions (`SUM`, `IF`, and the reference
functions).

- [x] `rule:core-classes/spreadsheet-evaluation-and-roster`: the parser covers references, ranges, sheets, names, structured references, constants, operators and calls. Its reference tokens are the shifter's. `extensions/spreadsheet/src/eval/parse.rs:1`, `extensions/spreadsheet/src/template/shift.rs:44`
- [x] The grid, the dependency order, the circular check, and the list of what was not computed. `extensions/spreadsheet/src/eval/mod.rs:1`
- [x] Values, coercion and errors by Excel's rules, with the `CellError` cases. `extensions/spreadsheet/src/eval/value.rs:1`
- [x] The function table with its first entries. `extensions/spreadsheet/src/eval/functions.rs:1`
- [x] The tests the `evaluator-core` check names. `extensions/spreadsheet/tests/evaluate.rs:1`

### 10. the function set, and Reader::evaluate

**Does:** the whole function set, and a program calls `$reader->evaluate()`.

**Split:** the functions are three disjoint batches once the table's shape has landed: math, logic
and information; text; and lookup, date and time. Each goes to a writing subagent with its own
file under `src/eval/`. The table is one line per function, which the session adds. None of them
builds.

- [x] `rule:core-classes/spreadsheet-evaluation-and-roster`: every function of the set, `TEXT` through the number-format renderer. `extensions/spreadsheet/src/eval/functions.rs:1`, `extensions/spreadsheet/src/format/mod.rs:401`
- [x] The `reader-evaluate` export and the overlay that `reader-rows` reads computed results from. `wit/spreadsheet.wit:637`, `extensions/spreadsheet/src/lib.rs:1339`, `extensions/spreadsheet/src/read/mod.rs:794`, `extensions/spreadsheet/manifest.json:90`
- [x] Every format feeds the evaluator: xlsx text, xls and xlsb tokens through `tokens.rs`, and ods through `openformula.rs`. `extensions/spreadsheet/src/read/mod.rs:1004`
- [x] `Reader::evaluate` in the Novis half, and a CSV returns `[]`. `extensions/spreadsheet/nvs/Reader.nvs:134`
- [ ] The tests and cases the `evaluator-functions` and `reader-evaluate-runs` checks name. `extensions/spreadsheet/tests/evaluate.rs:1`, `tests/conformance/novis/spreadsheet-evaluate-computes-a-report-it-wrote.nvst:1`, `tests/conformance/novis/spreadsheet-evaluate-lists-what-it-could-not-compute.nvst:1`
- [ ] Feature proofs for `Novis\Spreadsheet\Reader::evaluate`: `about.md`, three examples, the bench and the attack (a workbook of circular references and deep chains). `docs/examples/novis/Spreadsheet-Reader/evaluate/about.md:1`, `benches/members/novis/Spreadsheet-Reader/evaluate.nvs:1`, `tests/hostile/novis/Spreadsheet-Reader/evaluate/01-a-workbook-that-never-stops-computing.nvs:1`
- [x] The fragment states the call, the set, the bounds and the costs. Its Not-shipped line keeps only writing ods. The roster's `Reader` row names `evaluate`. Delete the record. `docs/rules/core-classes/spreadsheet-evaluation-and-roster.md:23`, `docs/rules/core-classes/spreadsheet-roster.md:10`

### 11. Workbook writes an ods with values, styles, columns and formulas

**Does:** a name ending `.ods` writes a valid ods that calamine and our reader read back.

- [ ] `rule:core-classes/spreadsheet-evaluation-and-roster`: the ods writer behind the `writer` resource: the held workbook, the package parts, and the zip framing reused. `extensions/spreadsheet/src/ods/mod.rs:1`, `extensions/spreadsheet/src/writer.rs:256`, `extensions/spreadsheet/src/zip.rs:1`
- [ ] Each value type as its `office:value-type`, rich cells as `text:span`, and styles once for each combination. `extensions/spreadsheet/src/ods/cells.rs:1`, `extensions/spreadsheet/src/styles.rs:1`
- [ ] Columns, the header defaults, freeze, defined names and formulas through `openformula.rs`. `extensions/spreadsheet/src/ods/mod.rs:1`, `extensions/spreadsheet/src/openformula.rs:1`
- [ ] `writer-open` takes the format, and `Workbook.nvs` accepts `ods` by name and option. `extensions/spreadsheet/src/lib.rs:1083`, `wit/spreadsheet.wit:190`, `extensions/spreadsheet/nvs/Workbook.nvs:253`, `extensions/spreadsheet/nvs/Workbook.nvs:291`
- [ ] The tests and the case the `ods-writer-core` and `ods-writer-runs` checks name. `extensions/spreadsheet/tests/ods.rs:1`, `tests/conformance/novis/spreadsheet-writes-ods-from-the-same-calls.nvst:1`
- [ ] A fourth example for `Workbook::createForFile` and `Workbook::fromRows` writes an ods. The writes-as-a-stream fragment states ods's held workbook and its cost. `docs/rules/core-classes/spreadsheet-writes-as-a-stream.md:66`

### 12. an ods carries every sheet-end part but charts

**Does:** each sheet-end part a program adds to an ods is written, in its ODF place.

- [ ] `rule:core-classes/spreadsheet-sheet-end-parts`: merges, links and notes in their cells, and validation before the first sheet. `extensions/spreadsheet/src/ods/mod.rs:1`, `extensions/spreadsheet/src/tail.rs:1`
- [ ] Hidden rows, columns and sheets, outline groups, print setup and protection. `extensions/spreadsheet/src/ods/mod.rs:1`
- [ ] Tables as database ranges, and conditional formats as `calcext:conditional-formats`. `extensions/spreadsheet/src/table.rs:1`, `extensions/spreadsheet/src/conditional.rs:1`
- [ ] Images as `draw:frame` in their anchor cell, with the picture in the package. `extensions/spreadsheet/src/drawing.rs:1`
- [ ] The tests the `ods-writer-parts` check names. `extensions/spreadsheet/tests/ods.rs:1`

### 13. an ods carries charts, the component's size is recorded, and no rule owes anything

**Does:** charts reach ods, the size is re-recorded, and every check of the goal but the gate's
passes.

- [ ] `rule:core-classes/spreadsheet-sheet-end-parts`: each chart as `Object N/content.xml` with `chart:chart`, its frame in the anchor cell, and its manifest entry. `extensions/spreadsheet/src/ods/chart.rs:1`, `extensions/spreadsheet/src/chart.rs:1`
- [ ] The tests the `ods-writer-charts` check names. `extensions/spreadsheet/tests/ods.rs:1`
- [ ] The evaluation fragment's Not-shipped line goes, and its roster paragraph says ods writes. The module doc's second-wave sentence goes. Delete the record. `docs/rules/core-classes/spreadsheet-evaluation-and-roster.md:35`, `extensions/spreadsheet/src/lib.rs:38`
- [ ] `cargo test --test builtin the_recorded_growth_matches_the_embedded_components` runs once. The size it names goes into the module doc. `extensions/spreadsheet/src/lib.rs:33`, `crates/nvs-ext/tests/builtin.rs:163`
- [ ] `git grep -n -E "\*\*(Not shipped|Not on disk)\." -- "docs/rules/core-classes/spreadsheet-*.md"` finds nothing. `bun nv rules --render` and `bun nv render --website` are run. `docs/rules/core-classes/spreadsheet-has-no-io.md:74`
- [ ] `bun nv owners --closes spreadsheet-second-wave` passes. `tools/nv/cmd/owners.ts:1`

### 14. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 10 is built and the workspace builds with no warning; nothing of it has run (context budget).
Items 1 to 4 and 7 are ticked.

- **Functions**: 104 sorted rows in `eval/functions.rs:22`, from `eval/math.rs`, `text.rs`,
  `lookup.rs` and `date.rs` (subagent-written). Inputs a batch cannot match Excel on are listed;
  each file's module doc names them. New `Ctx` and `Book` helpers are in `eval/mod.rs`.
- **Feeding**: `read/evaluate.rs` builds the `Book` (names, `table_rows` with header and totals
  counts) and `Reader::overlay` gives `rows()` the computed results. An undecodable formula is
  `Some("")` in xls, xlsb and ods. xlsx shared formulas now move to each cell (`eval/moved.rs`).
- **Export**: `reader-evaluate` in the WIT, guest, manifest, typed table, both export lists, and
  `Reader::evaluate` in `Reader.nvs`.
- **Rule**: fragment and roster rewritten. The gap `spreadsheet-evaluate-and-write-ods` is narrowed
  to writing ods, not deleted: stages 11 to 13 still owe it. `bun nv rules --render` not run.
- **Written, never run**: `tests/evaluate.rs` and `tests/evaluate/*.rs`, the two conformance
  cases (their expected output is partly a guess), and the proofs (no `.out` files yet).
- The sweep's `env-secret-proofs` red is stale: no such check exists.

### Next

Finish stage 10, then stage 11.

1. Run `cargo test --test evaluate` in `extensions/spreadsheet` once, and fix each case that
   fails. A wrong Excel expectation in a case is fixed against Excel's documented behaviour; a
   function that cannot match is listed for that input, never computed differently.
   `a_computed_result_reads_by_the_read_table` is still owed: write it in
   `crates/nvs-ext/tests/spreadsheet_read.rs` or in `tests/evaluate.rs` through `Reader::evaluate`
   (a date-formatted formula cell reads back as a date, and a cell `rows()` skipped before reads).
2. `bun nv relink --build`, then run the two `reader-evaluate-runs` cases with `bun nv suite
   tests/conformance/ --case <path>`, and fix the cases or `Reader.nvs` to what the code gives.
3. Produce the three examples' `.out` files, then `bun nv proofs --id
   'Novis\Spreadsheet\Reader::evaluate'` and `bun nv proofs --comments` on the new `.nvs` files.
   Tick items 5 and 6.
4. `bun nv rules --render`, and set stage 10 done once `evaluator-functions` and
   `reader-evaluate-runs` pass.

Tests that may go red at the gate: from stage 10, `crates/nvs-cli/tests/meta.rs` and
`crates/nvs-stdlib/tests/ext_world.rs`, whose export lists now name `readerEvaluate` and
`reader-evaluate` (the order may be wrong); `crates/nvs-ext/tests/spreadsheet_read.rs` tests that
read an xls, xlsb or ods formula the decoder cannot read, since such a cell now carries empty text
inside the reader (hidden from `formulas: true`); a test that reads a table's records through
`binary::table`, which now returns four fields. From earlier stages, as the last handoff said:
`meta.rs`'s enum and Engine lists, `spreadsheet_reader.rs`'s `Reader::Visibility`, `ext_world.rs`'s
`source-bytes`, and a test of `Template::fill`'s help text.

### Backlog

- The `self::` panic in a generator body, `crates/nvs-ir/src/lower/mod.rs:2878`: its own gap, owned outside this goal. A generator body calls a static helper as `Reader::f()` until then.
- The bullets of `extensions/spreadsheet/src/lib.rs:38`'s **Not done yet.** list after `:53`: owned as `release-hygiene`'s records say.
