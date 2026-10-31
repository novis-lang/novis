---
looper: 1
title: the spreadsheet and PDF extensions have no known fault left in their module docs
summary: "every Not done yet bullet in extensions/spreadsheet/src/lib.rs, extensions/pdf/src/lib.rs and extensions/pdf/src/layout/table.rs is fixed and tested, or stated as a deliberate limit, and the lists are gone."
context: [tests/hostile/README.md]
stages:
  - id: 1
    title: "a defined name, a sheet name and a formula follow Excel's rules, and no written reference leaves the workbook"
    status: done
    context: [extensions/spreadsheet/src/writer.rs, extensions/spreadsheet/src/chart.rs, extensions/spreadsheet/src/conditional.rs, docs/rules/core-classes/spreadsheet-output-is-inert.md, docs/rules/core-classes/spreadsheet-bulk-boundary.md]
  - id: 2
    title: "a reader reads only the file it opened, under its bounds, and bytes that are no spreadsheet throw"
    status: open
    context: [extensions/spreadsheet/src/read/mod.rs, extensions/spreadsheet/src/read/zip.rs, extensions/spreadsheet/nvs/Reader.nvs, docs/rules/core-classes/spreadsheet-reads-as-a-stream.md, docs/rules/core-classes/decompression-bound.md]
  - id: 3
    title: "rows, sheets, merges and notes read what the file has"
    status: open
    context: [extensions/spreadsheet/src/read/mod.rs, extensions/spreadsheet/src/read/merge.rs, extensions/spreadsheet/src/read/extras.rs, extensions/spreadsheet/nvs/Reader.nvs, crates/nvs-ext/tests/spreadsheet_read.rs]
  - id: 4
    title: "sheet-end parts, addresses and values are checked once, the spreadsheet list is gone, and its size is recorded"
    status: open
    context: [extensions/spreadsheet/src/drawing.rs, extensions/spreadsheet/src/tail.rs, extensions/spreadsheet/nvs/Sheet.nvs, extensions/spreadsheet/nvs/Workbook.nvs, docs/rules/core-classes/spreadsheet-sheet-end-parts.md]
  - id: 5
    title: "a render inside a header keeps its own first pass, and nesting and decoding stop at named bounds"
    status: open
    context: [extensions/pdf/src/lib.rs, wit/pdf.wit, extensions/pdf/nvs/Document.nvs, extensions/pdf/src/read.rs, docs/rules/core-classes/pdf-header-is-a-page-function.md]
  - id: 6
    title: "a reader keeps its file in the engine, page strings match as written, and a second reader checks the passwords"
    status: open
    context: [extensions/pdf/nvs/Reader.nvs, extensions/pdf/src/read.rs, extensions/pdf/src/merge.rs, crates/nvs-ext/tests/pdf_standards.rs, docs/rules/core-classes/pdf-reader-and-merge.md]
  - id: 7
    title: "a table is as wide as its columns, splits a tall rowspan, aligns its cells and draws its rows"
    status: open
    context: [extensions/pdf/src/layout/table.rs, extensions/pdf/tests/tables.rs, docs/rules/core-classes/pdf-tables-across-pages.md]
  - id: 8
    title: "a mark, a string, a running element, an id and a heading are on the page they are drawn on"
    status: open
    context: [extensions/pdf/src/layout/block.rs, extensions/pdf/src/layout/inline.rs, extensions/pdf/src/paginate/mod.rs, extensions/pdf/src/layout/margin.rs, docs/rules/core-classes/pdf-css-subset.md]
  - id: 9
    title: "page breaks keep what avoid asks for, and a left or right page sets its own line widths"
    status: open
    context: [extensions/pdf/src/layout/block.rs, extensions/pdf/src/layout/text.rs, extensions/pdf/src/paginate/mod.rs, extensions/pdf/tests/paginate.rs]
  - id: 10
    title: "absolute and transformed boxes, the grid shorthands, elliptical radii and repeating gradients"
    status: open
    context: [extensions/pdf/src/layout/block.rs, extensions/pdf/src/layout/grid.rs, extensions/pdf/src/layout/boxes.rs, extensions/pdf/src/write.rs, extensions/pdf/subset.json]
  - id: 11
    title: "flex items, right-to-left and SVG text are laid out as HTML's, clippy is clean, and the PDF lists are gone"
    status: open
    context: [extensions/pdf/src/layout/flex.rs, extensions/pdf/src/layout/inline.rs, extensions/pdf/src/fonts.rs, crates/nvs-ext/tests/builtin.rs, tools/nv/cmd/owners.ts]
  - id: 12
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: names-and-references
    stage: 1
    name: "extensions/spreadsheet: a defined name takes only a range of this workbook, and names, sheet names and formulas follow Excel's limits"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, write]
    tests: [define_takes_only_a_range_of_this_workbook, define_throws_for_a_dde_or_external_reference, a_defined_name_past_255_characters_or_in_r1c1_form_throws, a_defined_names_sheet_that_never_comes_throws_at_finish, a_sheet_name_with_a_control_character_or_named_history_throws, a_formula_that_is_empty_or_past_8192_characters_throws]
  - id: chart-ranges
    stage: 1
    name: "extensions/spreadsheet: a chart range stays in the workbook, its sheet name follows the sheet name rule, and a title has Excel's limit"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, chart]
    tests: [a_series_range_into_another_workbook_or_a_url_throws, a_series_range_sheet_name_follows_the_sheet_name_rule, a_series_range_with_an_unclosed_quote_or_two_sheets_throws, a_chart_title_past_255_characters_throws]
  - id: conditional-formula-bound
    stage: 1
    name: "extensions/spreadsheet: a conditional format's formula has Excel's formula limit"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, sheet_end]
    tests: [a_conditional_formula_past_8192_characters_throws]
  - id: names-and-references-runs
    stage: 1
    name: "a program sees define and a chart range throw for a reference outside the workbook"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-define-throws-for-a-reference-outside-the-workbook.nvst, --case, tests/conformance/novis/spreadsheet-a-chart-range-outside-the-workbook-throws.nvst, --case, tests/conformance/novis/spreadsheet-define-writes-a-name-excel-allows.nvst]
    tests: [tests/conformance/novis/spreadsheet-define-throws-for-a-reference-outside-the-workbook.nvst, tests/conformance/novis/spreadsheet-a-chart-range-outside-the-workbook-throws.nvst]
  - id: reader-bounds
    stage: 2
    name: "nvs-ext: a reader call with another source throws, max 0 throws, properties read once, and bound and locale errors are plain"
    run: [cargo, test, --test, spreadsheet_read]
    tests: [a_reader_call_with_another_source_throws, rows_with_max_0_throws, properties_are_read_once_per_reader, a_decompression_bound_message_names_no_rule, an_unknown_locale_throws_naming_it]
  - id: reader-bounds-runs
    stage: 2
    name: "a program sees bytes that are no spreadsheet and an unknown locale throw"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-bytes-that-are-not-a-spreadsheet-throw.nvst, --case, tests/conformance/novis/spreadsheet-an-unknown-locale-throws.nvst]
    tests: [tests/conformance/novis/spreadsheet-bytes-that-are-not-a-spreadsheet-throw.nvst, tests/conformance/novis/spreadsheet-an-unknown-locale-throws.nvst]
  - id: reader-reads-what-the-file-has
    stage: 3
    name: "nvs-ext: formula rows, sheet sizes, merged fills and note text read back, and ids 14 and 22 render as Excel shows them"
    run: [cargo, test, --test, spreadsheet_read]
    tests: [a_row_of_formulas_without_cached_values_reads_back, sheets_counts_a_sheet_with_no_dimension, sheets_reads_each_sheet_once_per_reader, a_merged_fill_copies_the_text, a_note_decodes_its_escapes, builtin_formats_14_and_22_render_as_excel_shows_them_in_en_us]
  - id: reader-reads-what-the-file-has-runs
    stage: 3
    name: "a totals row of formulas reads back, and a sheet name from sheets() reads its rows"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-reads-back-a-totals-row-of-formulas.nvst, --case, tests/conformance/novis/spreadsheet-a-sheet-name-from-sheets-reads-its-rows.nvst]
    tests: [tests/conformance/novis/spreadsheet-reads-back-a-totals-row-of-formulas.nvst, tests/conformance/novis/spreadsheet-a-sheet-name-from-sheets-reads-its-rows.nvst]
  - id: sheet-end-parts
    stage: 4
    name: "extensions/spreadsheet: a cut image, an image past the grid, a whole rule's limits and a second print are checked"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, sheet_end]
    tests: [an_image_cut_short_throws, an_image_that_ends_past_the_grid_throws, a_whole_number_rule_writes_no_text_limits, a_second_print_on_a_sheet_throws]
  - id: writer-values
    stage: 4
    name: "extensions/spreadsheet: a header with autofilter gets its filter name, and a column past 32 bits is named in full"
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, write]
    tests: [a_header_with_autofilter_writes_the_filter_name, a_column_past_32_bits_is_named_in_full]
  - id: addresses-and-values-runs
    stage: 4
    name: "a program sees a key or a column asked for too early throw, NaN throw for CSV, an iterator export written once, and a finished sheet named"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-a-key-before-the-first-keyed-row-throws.nvst, --case, tests/conformance/novis/spreadsheet-a-column-before-its-first-data-row-throws.nvst, --case, tests/conformance/novis/spreadsheet-nan-throws-for-csv-as-for-xlsx.nvst, --case, tests/conformance/novis/spreadsheet-an-export-of-an-iterator-writes-once.nvst, --case, tests/conformance/novis/spreadsheet-a-cell-on-a-finished-sheet-names-the-sheet.nvst]
    tests: [tests/conformance/novis/spreadsheet-a-key-before-the-first-keyed-row-throws.nvst, tests/conformance/novis/spreadsheet-a-column-before-its-first-data-row-throws.nvst, tests/conformance/novis/spreadsheet-nan-throws-for-csv-as-for-xlsx.nvst, tests/conformance/novis/spreadsheet-an-export-of-an-iterator-writes-once.nvst, tests/conformance/novis/spreadsheet-a-cell-on-a-finished-sheet-names-the-sheet.nvst]
  - id: spreadsheet-attacks
    stage: 4
    name: "every spreadsheet attack this goal changed passes"
    run: [bun, nv, proofs, --run, --only, 'Novis\Spreadsheet\Workbook::define', 'Novis\Spreadsheet\Workbook::sheet', 'Novis\Spreadsheet\Formula::of', 'Novis\Spreadsheet\Sheet::chart', 'Novis\Spreadsheet\Sheet::conditional', 'Novis\Spreadsheet\Sheet::image', 'Novis\Spreadsheet\Sheet::print', 'Novis\Spreadsheet\Sheet::cell', 'Novis\Spreadsheet\Sheet::column', 'Novis\Spreadsheet\Export::bytes', 'Novis\Spreadsheet\Reader::open', 'Novis\Spreadsheet\Reader::rows', 'Novis\Spreadsheet\Reader::sheets', 'Novis\Spreadsheet\Reader::notes']
  - id: spreadsheet-size-recorded
    stage: 4
    name: "nvs-ext: the spreadsheet component's recorded size matches the bytes the binary carries"
    run: [cargo, test, --test, builtin, the_recorded_growth_matches_the_embedded_components]
    tests: [the_recorded_growth_matches_the_embedded_components]
  - id: pdf-nesting-and-decoding-bounds
    stage: 5
    name: "extensions/pdf: nesting past the depth bound and a stream or attachment past the decompression bound throw"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, render]
    tests: [layout_nested_past_the_depth_bound_throws, an_invoice_xml_nested_past_the_depth_bound_throws]
  - id: pdf-decoding-bounds
    stage: 5
    name: "extensions/pdf: a stream or an attachment past the decompression bound throws"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, read]
    tests: [a_stream_past_the_decompression_bound_throws, an_attachment_past_the_decompression_bound_throws]
  - id: pdf-nested-render-runs
    stage: 5
    name: "a header function that renders another document leaves the outer render whole"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/pdf-a-header-may-render-another-document.nvst]
    tests: [tests/conformance/novis/pdf-a-header-may-render-another-document.nvst]
  - id: pdf-reader-and-strings
    stage: 6
    name: "extensions/pdf: a reading holds its file, and a predefined CMap gives text"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, read]
    tests: [text_reads_the_file_its_reading_holds, a_predefined_cmap_gives_unicode_text]
  - id: pdf-second-reader
    stage: 6
    name: "nvs-ext: a second reader opens an encrypted file with its user password and checks its Perms"
    run: [cargo, test, --test, pdf_standards]
    tests: [a_second_reader_opens_the_encrypted_file_with_the_user_password, a_second_reader_checks_the_perms_entry]
  - id: pdf-page-string-runs
    stage: 6
    name: "Page::string matches a string's name as the CSS wrote it"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/pdf-page-string-matches-the-name-as-written.nvst]
    tests: [tests/conformance/novis/pdf-page-string-matches-the-name-as-written.nvst]
  - id: pdf-tables
    stage: 7
    name: "extensions/pdf: a tall rowspan splits, an auto table is as wide as its columns, cells align, rows draw, and a table in a row gets a cell"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, tables]
    tests: [a_rowspan_piece_taller_than_a_page_splits_between_lines, a_table_without_a_width_is_as_wide_as_its_columns, a_cells_vertical_align_places_its_content, a_rows_and_a_groups_background_is_drawn, collapsed_row_and_group_borders_are_drawn, a_table_directly_in_a_row_is_wrapped_in_a_cell]
  - id: pdf-tables-runs
    stage: 7
    name: "a program's table without a width is as wide as its content"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/pdf-a-table-is-as-wide-as-its-content.nvst]
    tests: [tests/conformance/novis/pdf-a-table-is-as-wide-as-its-content.nvst]
  - id: pdf-page-attribution
    stage: 8
    name: "extensions/pdf: marks, strings, running elements and ids inside a cell, inline-block, float or flex item are on their page, and counters read as CSS says"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, paginate]
    tests: [a_mark_or_string_in_a_table_cell_is_on_the_page_its_row_is_drawn_on, a_string_or_running_element_in_an_inline_block_float_or_flex_item_is_on_its_page, an_id_in_an_inline_block_float_flex_item_cell_or_running_element_has_its_page, target_counter_reads_any_counter, a_margin_box_reads_a_named_counter, element_start_and_first_except_read_as_css_says]
  - id: pdf-bookmarks
    stage: 8
    name: "extensions/pdf: a heading inside an inline-block, float, flex item or cell has a bookmark"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, render]
    tests: [a_heading_in_an_inline_block_float_flex_item_or_cell_has_a_bookmark]
  - id: pdf-breaks
    stage: 9
    name: "extensions/pdf: left and right pages set their widths, avoid keeps every box kind, and a float moves to the next page"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, paginate]
    tests: [a_left_or_right_page_of_another_width_sets_its_own_line_widths, break_inside_avoid_keeps_a_row_a_flex_box_and_an_inline_block_whole, break_after_and_break_before_avoid_keep_two_boxes_together, a_float_in_a_paragraph_is_placed_at_its_line_and_moves_to_the_next_page]
  - id: pdf-boxes
    stage: 10
    name: "extensions/pdf: left with right stretches, a transformed box contains, the grid shorthands read, and radii and repeating gradients draw"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, layout]
    tests: [an_absolute_box_with_left_and_right_and_no_width_stretches, a_transformed_box_is_a_containing_block, the_grid_template_and_grid_shorthands_are_read, elliptical_radii_are_drawn, repeating_gradients_are_drawn]
  - id: pdf-transformed-link
    stage: 10
    name: "extensions/pdf: a link in a transformed box covers the box as drawn"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, render]
    tests: [a_link_in_a_transformed_box_covers_its_transformed_box]
  - id: pdf-flex-and-text
    stage: 11
    name: "extensions/pdf: before, after and br are flex items, baseline and auto margins work, and an inline box in a right-to-left line goes right to left"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, layout]
    tests: [before_after_and_br_are_flex_items, flex_items_align_on_their_first_baseline, an_auto_margin_on_a_flex_item_takes_the_free_space, an_inline_box_in_a_right_to_left_line_is_placed_right_to_left]
  - id: pdf-svg-fonts
    stage: 11
    name: "extensions/pdf: an SVG font-family maps to the Liberation face HTML maps it to"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, render]
    tests: [an_svg_font_family_maps_as_html_does]
  - id: pdf-clippy-clean
    stage: 11
    name: "the PDF component has no clippy warning"
    run: [cargo, clippy, --manifest-path, extensions/pdf/Cargo.toml, --all-targets, --, -D, warnings]
  - id: pdf-attacks
    stage: 11
    name: "every PDF attack this goal changed passes"
    run: [bun, nv, proofs, --run, --only, 'Novis\Pdf\Document::header', 'Novis\Pdf\Document::fromHtml', 'Novis\Pdf\Document::render', 'Novis\Pdf\Document::invoiceXml', 'Novis\Pdf\Reader::open', 'Novis\Pdf\Reader::text', 'Novis\Pdf\Pages::merge', 'Novis\Pdf\Page::string']
  - id: pdf-size-recorded
    stage: 11
    name: "nvs-ext: the PDF component's recorded size matches the bytes the binary carries"
    run: [cargo, test, --test, builtin, the_recorded_growth_matches_the_embedded_components]
    tests: [the_recorded_growth_matches_the_embedded_components]
  - id: no-module-doc-owes-anything
    stage: 11
    name: "no Not done yet list is left in the spreadsheet and PDF module docs"
    run: [git, grep, -n, -F, "**Not done yet.**", --, extensions/spreadsheet/src/lib.rs, extensions/pdf/src/lib.rs, extensions/pdf/src/layout/table.rs]
    exit: nonzero
    stdout_exact: ""
  - id: owners-closes
    stage: 11
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, extension-leftovers]
    reach: true
    floor: false
---

# the spreadsheet and PDF extensions have no known fault left in their module docs

## Why

The first release ships with no known fault. The spreadsheet and PDF components each end their
module doc with a **Not done yet.** list, and `layout::table` has a third. Two items there are
security faults: `Workbook::define` writes a DDE-shaped reference such as `cmd|'/c calc'!A0`, and
the reader exports read any `source` they are handed. The rest are wrong results, missing bounds,
messages that name a rule id, and CSS the layout reads wrong.

`release-hygiene` turned the three lists into three gap records under `data/gaps/` that name this
goal: `nvs-spreadsheet/spreadsheet-reader-and-writer-leftovers`, `nvs-pdf/pdf-leftovers` and
`nvs-pdf/pdf-table-leftovers`. Each bullet is fixed with a test, or, where the behaviour is right,
stated as a limit in its rule. The item that clears the last bullet of a list deletes that list's
record.

## Standing decisions

**The gates.** Stage 1 starts when `docs/goals/spreadsheet-second-wave.md` is gone (that goal
retired), `bun nv owners --check` exits 0, and `bun nv owners --closes extension-leftovers` lists
the three records. Stage 5 needs only the last two. A list with no record gets one from the first
session that touches it, in its first commit. Home: the item's commit.

**Scope.** The bullets of `extensions/spreadsheet/src/lib.rs:38`'s list from `:53` to `:92`, all
nine bullets of `extensions/pdf/src/lib.rs:45`'s list, and the six faults of
`extensions/pdf/src/layout/table.rs:62`. `spreadsheet-second-wave` owns the bullets before `:53`,
the ods number style (its stage 4) and the xlsb, xls and ods extras (its stages 4 to 7). Its
stages delete those bullets, so the line numbers here move. A session finds each bullet by its
words. The `notes` escapes share the extras bullet: if that goal deleted the bullet, the fix is
still owed here.

**What stays.** The spreadsheet `Cost:` bullet (`extensions/spreadsheet/src/lib.rs:93`) and the
table module's `Cost:` paragraph (`extensions/pdf/src/layout/table.rs:68`) are trade-offs, not
faults. They stay as plain paragraphs, without the bold lead. When a list's last bullet goes, its
`**Not done yet.**` lead goes too. A sentence another goal still owes is reworded as a pointer to
its rule, with no bold lead.

**Fix or limit, per bullet.** Every bullet is a fix, except these four, which are limits:
- **Built-in number formats 14 and 22** render as Excel shows them in en-US, `m/d/yyyy` and
  `m/d/yyyy h:mm`, whatever `locale` is. SheetJS's `m/d/yy` is not the reference: Excel is. The
  fragment states it beside the `locale` row (`docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:54`),
  and a test pins it.
- **A word of two million characters, and a page of a million `q` operators**, reach the PDF
  component's memory limit, and the call throws as every guest crash does
  (`rule:packaging/a-guest-crash-throws`). The cost grows with the input, so no bound below the
  memory limit is fairer. `rule:core-classes/pdf-one-engine` states the limit, its size, and the
  error. Safe fallback: if the session finds a copy or a quadratic step behind either case, it fixes
  that step, and the case stays a limit only if it still ends at the limit.
- **100,000 nested `div`s** are the same limit if, after the depth bound below, the case still
  reaches the memory limit. Otherwise the depth bound covers it.
- **`merge`'s cost** (64 ms and 4.4 MB for three small pages) is measured once on
  `benches/members/novis/Pdf-Pages/merge.nvs`. If one step holds more than half the time, that step
  is fixed. What remains is a `Cost:` sentence in `rule:core-classes/pdf-reader-and-merge`.

**`Workbook::define` takes a range of this workbook, and nothing else.** The reference is an
optional sheet name, quoted as Excel quotes one, then `!`, then an A1 cell, an A1 range, a whole
column range (`$A:$C`) or a whole row range (`$1:$3`), each with or without `$`. A `|`, a `[`, a
URL, a function call, an R1C1 reference or anything else throws `LogicError`, which shows a valid
example. The sheet name follows the sheet name rule below. A sheet the workbook never adds throws at
`finish()`, naming the name and the sheet. This closes DDE and external links through a name.
Declined, written into `rule:core-classes/spreadsheet-output-is-inert`: a constant or a formula as a
name's value. `Formula` is the one way to write a formula. The conformance case
`tests/conformance/novis/spreadsheet-define-writes-a-name-excel-allows.nvst` defines `Odd` as
`Shop!$A$1<x>&`, which now throws. Its source changes to a quoted sheet name with `<`, `>` and `&`,
so it still proves the escape, and its expected output keeps the escape line.

**Excel's limits are the bounds** (`rule:core-classes/spreadsheet-bulk-boundary`: Excel's own grid
and limits are the only limits the component adds):
- A defined name: at most 255 characters. A name in A1 or R1C1 form (`R1C1`, `R`, `C`, `RC2`) throws.
- A sheet name: 1 to 31 characters, none of `[ ] : * ? / \`, no control character (NUL included),
  no `'` first or last, and not `History` in any case. One function checks it,
  `check_sheet_name` (`extensions/spreadsheet/src/writer.rs:165`). `Workbook::sheet`, `define` and a
  chart's series range all call it.
- A formula: 1 to 8,192 characters, after the leading `=` is dropped. One Rust check covers every
  formula the writer writes: `Formula::of` cells, column formulas, totals, conditional formats and
  validation formulas. `Formula::of('')` throws in the Novis half too.
- A chart title and an axis title: at most 255 characters.
- An image or a chart that would end past column `XFD` or row 1,048,576 throws, naming the cell it
  would end at. This bounds the 10,000,000-pixel width.

**A chart's series range** uses the reference parser `define` uses, without the whole-column and
whole-row forms. Another workbook (`[other.xlsx]Shop!B2:B3`), a URL, an unclosed quote and a second
sheet name (`Shops!Shops!B2:B3`) each throw `LogicError` naming the range.

**A reader reads only the file it opened.** `read::Reader::open` keeps the source's size and a
64-bit FNV-1a hash of its first 512 bytes and its last 512 bytes. Every later call checks them
first, and a different source throws `Invalid`. The check lives in the core `Reader`, so a host
test reaches it. No new crate: FNV-1a is a few lines. Cost: two reads of 512 bytes per call, on a
path that reads at least a batch of rows. `rule:core-classes/spreadsheet-reads-as-a-stream` states
it. Safe fallback: if the size alone differs on every mismatch the tests can build, the hash still
stays.

**The other reader bounds.**
- `readerRows` with `max` 0 throws `Invalid`.
- `readerProperties` reads the properties once per reader, keeps them, and counts against the byte
  bound once. Memory: the properties' text, per reader.
- `sheets()` reads each sheet's facts once per reader and keeps them. A sheet with no
  `<dimension>`, or one that says `A1` over a sheet with more cells, is counted from its rows on
  that first read. Memory: a few numbers per sheet, per reader.
- An unknown `locale` throws `LogicError` naming it (`extensions/spreadsheet/src/format/locale.rs:82`).
- The decompression-bound messages (`extensions/spreadsheet/src/read/zip.rs:79`, `:86`) say what
  was too large and which option raises it, and name no rule.

**Bytes that are not a spreadsheet throw.** A source opens as CSV when its bytes are not a zip or a
compound file and its first 4 KiB have no NUL byte and no C0 control character other than tab, CR,
LF and form feed. Anything else that is not a workbook throws `ParseError`, naming the formats the
reader reads. A UTF-16 BOM is text, as `spreadsheet-second-wave` decides. Safe fallback: if a real
CSV fixture of the tree has another control character, the test is the NUL byte alone.

**A sheet name from `sheets()` is accepted back.** Every `Reader` method that takes a sheet takes
`tainted string|int` (`extensions/spreadsheet/nvs/Reader.nvs:71`). The name is only compared with
the file's sheet names, and never written anywhere.

**Reading values.**
- A row whose cells are all formulas with no cached value is a row. Each such cell reads as `null`,
  or as its `Formula` with `cached` `null` under `formulas: true`. Only a row with no cell at all
  is skipped (`extensions/spreadsheet/src/read/mod.rs:884`).
- A merged fill copies the top-left cell's `text` with its value (`extensions/spreadsheet/src/read/merge.rs:146`).
- `notes` decodes `_xHHHH_` escapes as cell text does (`extensions/spreadsheet/src/read/xml.rs:202`).

**Writer addresses and values.**
- A column letter is upper case, `A` to `XFD`. Any other string is a key. A key before the first
  keyed row throws `LogicError` naming the key (`extensions/spreadsheet/nvs/Workbook.nvs:421`). A
  program that wrote a lower-case letter writes it in upper case.
- `Sheet::column($key)` before the first data row throws `LogicError`, and so does the
  `writerColumn` export with no data row.
- `Sheet::cell` on a finished sheet throws a message that names the sheet and says it is finished
  (`extensions/spreadsheet/nvs/Workbook.nvs:324`).
- The `writerCell` and `writerColumn` exports check the column against 16,384 before any cast, and
  the message names the full number (`extensions/spreadsheet/src/lib.rs:1144`).
- A second `Sheet::print` on one sheet throws `LogicError`. A program states its page setup in
  one call. This matches the other parts that are set once.
- A `whole` validation rule writes whole-number bounds and no text-length limit.
- A sheet with `{header: true, autofilter: true}` writes `_xlnm._FilterDatabase` as `fromRows` does.
- NaN and the infinities throw `LogicError` for CSV as they do for xlsx, from one check in the
  Novis half's CSV path (`extensions/spreadsheet/nvs/Workbook.nvs:603`).
- An `Export` whose rows are an `Iterator` writes once. A second write throws `LogicError`, saying an
  iterator is read once. An array or an `Iterable` writes every time.

**A render inside a header keeps its own first pass.** The `KEPT` thread-local
(`extensions/pdf/src/lib.rs:430`) goes. `first` returns a `kept` WIT resource beside the page facts,
and `second` takes it. `Document.nvs` holds it between the two calls. The request's end drops any
resource left. Memory: one laid-out body per unfinished render, O(in-flight). The hostile case's
step 3 (`tests/hostile/novis/Pdf-Document/header/01-headers-that-throw-load-and-render-again.nvs:32`)
then renders both documents. Home: `rule:core-classes/pdf-header-is-a-page-function`.

**Nesting and decoding bounds in the PDF component.**
- **Layout depth.** Nested tables, flex and grid containers and inline-blocks recurse through
  `layout/text.rs:505`. The session measures the wasm stack used per level on the hostile case. The
  bound is the largest power of two that leaves half the stack free, and at least 32. Past it the
  render throws `LogicError`, naming the depth. Safe fallback: if 32 does not fit, the component's
  stack size is raised until it does, and the module doc states the memory it spends per render.
- **Invoice XML depth.** Past 256 nested elements, `invoiceXml` and `Reader::invoiceXml` throw
  `ParseError` (`extensions/pdf/src/factur_x.rs:83`).
- **Decompression.** Every stream the reader or `merge` decodes, attachments included, runs under
  `rule:core-classes/decompression-bound`'s ceiling and ratio (`extensions/pdf/src/read.rs:173`).
  Past it the call throws `ParseError` naming the bound. The hostile PDF bombs then end with that
  error, caught, and lose their `ends-early` line.

**Reading a PDF.** `Reader::open` returns a reader over a `reading` WIT resource that holds the file
in the engine, and `text` takes the resource (`wit/pdf.wit:166`). The Novis half keeps no copy.
Memory: the file once, per open reader, in place of one copy per `text` call.

**String names are case-sensitive**, as CSS custom identifiers are. `Page::string` stops lowercasing
its argument (`extensions/pdf/nvs/Page.nvs:36`), and the engine stops lowercasing the stored names
(`extensions/pdf/src/layout/margin.rs:63`, `extensions/pdf/src/paginate/mod.rs:286`).

**Passwords.** The second reader is `lopdf`, as a dev-dependency of `crates/nvs-ext` alone. It must
open AES-256 (revision 6) files with the user password. It never ships. Safe fallback: if the
installed `lopdf` cannot decrypt revision 6, the test decrypts `/Perms` with the file key by hand,
from ISO 32000-2's algorithm, and checks its flags. That code is written in the test, never shared
with `encrypt.rs`. **veraPDF** runs in CI's Linux job through `bun nv pdf-validate`. That command
renders the PDF/A-2b, PDF/A-3b and PDF/UA-1 fixtures with the built binary into `.agent-tmp/`, and
runs the `verapdf/cli` image, pinned by digest. Where Docker runs no Linux containers it says so and
exits 0, as the `requires: <service>` cases do. Home: `rule:core-classes/pdf-standards-and-passwords`.

**Predefined CMaps.** `read.rs`'s `cmap_resolver` (`extensions/pdf/src/read.rs:237`) resolves the
predefined CMaps through `hayro-cmap`'s embedded data, if the installed version has it. The
component's growth is recorded in stage 11. Safe fallback: if the installed version embeds none,
or the growth is over 3 MiB, it stays a limit. `rule:core-classes/pdf-reader-and-merge` then states
it, and its Declined list says why.

**The CSS the layout fixes** follows the CSS specifications the subset already cites, and each new
property or value is a line in `extensions/pdf/subset.json` (`rule:core-classes/pdf-css-subset-table`).
- A table without a `width` is as wide as its columns' widest content, up to its containing block.
- `vertical-align` on a cell takes `top`, `middle`, `bottom` and `baseline`. `baseline` aligns each
  cell's first line.
- A row's and a group's background is drawn under its cells. Their borders are drawn under
  `border-collapse: collapse` only, as CSS 2.1 says.
- A `display: table` element directly in a row is wrapped in an anonymous cell.
- A `rowspan` piece taller than a page splits between lines, as a tall row does.
- `break-after: avoid` and `break-before: avoid` keep two boxes on one page when they fit.
  `break-inside: avoid` keeps a table row, a flex container and an inline-block whole.
- `counter()` and `target-counter()` read any named counter. `element()` reads `start` and
  `first-except` as CSS GCPM says.
- A `margin: auto` on a flex item takes the free space. `align-items: baseline` aligns first
  baselines. `flex.rs:163` already parses `baseline`, so the session checks what is missing before
  it writes.

**Each fix that changes what a user sees** gets a conformance case or a hostile case. A hostile
case whose `ends-early` step no longer ends the program catches the new error and loses that line.
Each changed member's `about.md` states the new behaviour where it stated the old one.
`bun nv proofs --id '<member>'` owes nothing at the end of each item, and
`bun nv proofs --comments` passes on each changed `.nvs`.

**Text an end user reads.** Every new or changed error message, `about.md` and example comment
follows AGENTS.md § *Text an end user reads*.

**Rules.** No new rule is made. Each item adds its sentence to the fragment it names and its tests
to that rule's `guardedBy` in `data/rules/core-classes/<rule>.json`. A session then runs
`bun nv rules --render` once.

**Sizes.** Stage 4 and stage 11 each run `the_recorded_growth_matches_the_embedded_components` once,
by name. Each writes the size the failure names into its module doc
(`extensions/spreadsheet/src/lib.rs:33`, `extensions/pdf/src/lib.rs:39`) and states the growth in
its commit.

**Clippy.** The PDF lints are fixed, not allowed: the large variants of `layout::Block`
(`extensions/pdf/src/layout.rs:459`) and `inline::Step` (`extensions/pdf/src/layout/inline.rs:1242`)
are boxed. `bun nv verify` does not gain a clippy run over the component crates. Each would compile
a second workspace on every verify run. The `pdf-clippy-clean` check runs it once.

**Trade-offs, as a whole.** Security: no written name or chart range reaches outside the workbook, a
reader reads only its own file, and every PDF decode has a bound. Latency: two 512-byte reads per
reader call, and nothing else on any request path. Memory: a kept body per unfinished render, a
held file per PDF reader, and properties and sheet facts per spreadsheet reader, all per request.
Simplicity: one name check, one reference parser and one formula check, each called from every
site.

**Lanes.** Stages 1 to 4 touch only `extensions/spreadsheet/` and `crates/nvs-ext/tests/spreadsheet_read.rs`.
Stages 5 to 11 touch only `extensions/pdf/`, `wit/pdf.wit` and the PDF tests. The two lanes may run
side by side as side goals. Both re-record a size, but each writes its own module doc, so they share
no file. Inside a lane the stages run in order: 1 and 4 share `writer.rs` and `Workbook.nvs`, 2 and
3 share `read/mod.rs`, 5 and 6 share `lib.rs` and `read.rs`, 7 and 8 share `table.rs`, 8 to 10 share
`block.rs`, and 10 and 11 share `inline.rs`.

## Stages

### 1. a defined name, a sheet name and a formula follow Excel's rules, and no written reference leaves the workbook

**Does:** `define` writes only a range of this workbook, and every name, sheet name, formula and
chart range written is checked by one function each.

- [x] Gate: `docs/goals/spreadsheet-second-wave.md` is gone, `bun nv owners --check` exits 0, and `bun nv owners --closes extension-leftovers` lists the three records. `tools/nv/cmd/owners.ts:1`
- [x] `rule:core-classes/spreadsheet-output-is-inert`: one reference parser, and `define` takes only what it accepts. The sheet of each name is checked at `finish()`. `extensions/spreadsheet/src/writer.rs:317`, `extensions/spreadsheet/src/writer.rs:1214`, `extensions/spreadsheet/nvs/Workbook.nvs:211`
- [x] `rule:core-classes/spreadsheet-bulk-boundary`: a name past 255 characters and a name in R1C1 form throw. `extensions/spreadsheet/src/writer.rs:323`, `extensions/spreadsheet/src/writer.rs:328`
- [x] `rule:core-classes/spreadsheet-bulk-boundary`: `check_sheet_name` refuses control characters and `History`, and `Workbook::sheet` reaches it. `extensions/spreadsheet/src/writer.rs:165`, `extensions/spreadsheet/src/writer.rs:615`, `extensions/spreadsheet/nvs/Workbook.nvs:108`
- [x] `rule:core-classes/spreadsheet-formula-is-a-value`: one formula check, 1 to 8,192 characters, for every formula written, and `Formula::of('')` throws. `extensions/spreadsheet/src/cells.rs:330`, `extensions/spreadsheet/nvs/Formula.nvs:18`, `extensions/spreadsheet/src/conditional.rs:151`
- [x] `rule:core-classes/spreadsheet-sheet-end-parts`: a series range goes through the reference parser and `check_sheet_name`, and a title past 255 characters throws. `extensions/spreadsheet/src/chart.rs:208`, `extensions/spreadsheet/src/chart.rs:109`, `extensions/spreadsheet/src/chart.rs:300`, `extensions/spreadsheet/nvs/Chart.nvs:104`
- [x] The tests the `names-and-references`, `chart-ranges` and `conditional-formula-bound` checks name. `extensions/spreadsheet/tests/write.rs:1`, `extensions/spreadsheet/tests/chart.rs:1`, `extensions/spreadsheet/tests/sheet_end.rs:1`
- [x] The two new conformance cases, and the `Odd` line of the define case corrected. `tests/conformance/novis/spreadsheet-define-throws-for-a-reference-outside-the-workbook.nvst:1`, `tests/conformance/novis/spreadsheet-a-chart-range-outside-the-workbook-throws.nvst:1`, `tests/conformance/novis/spreadsheet-define-writes-a-name-excel-allows.nvst:14`
- [x] The `about.md` of `define`, `sheet` and `Formula::of` state the limits. The fragments state them, with the Declined bullet, and the module doc's names, charts and conditional sentences go. `docs/examples/novis/Spreadsheet-Workbook/define/about.md:1`, `docs/examples/novis/Spreadsheet-Formula/of/about.md:1`, `docs/rules/core-classes/spreadsheet-output-is-inert.md:1`, `extensions/spreadsheet/src/lib.rs:69`, `extensions/spreadsheet/src/lib.rs:83`

### 2. a reader reads only the file it opened, under its bounds, and bytes that are no spreadsheet throw

**Does:** every reader call checks its source, the reader's bounds are exact, and its errors say
what went wrong in plain words.

- [ ] `rule:core-classes/spreadsheet-reads-as-a-stream`: `read::Reader` keeps the source's size and hash, and every call checks them first. `extensions/spreadsheet/src/read/mod.rs:484`, `extensions/spreadsheet/src/lib.rs:1163`
- [ ] `readerRows` with `max` 0 throws, and `readerProperties` reads once per reader. `extensions/spreadsheet/src/lib.rs:1200`, `extensions/spreadsheet/src/lib.rs:1339`, `extensions/spreadsheet/src/read/mod.rs:793`
- [ ] `rule:core-classes/decompression-bound`: the two bound messages name no rule, and say what to raise. `extensions/spreadsheet/src/read/zip.rs:79`, `extensions/spreadsheet/src/read/zip.rs:86`
- [ ] `rule:core-classes/spreadsheet-reads-as-a-stream`: an unknown `locale` throws. `extensions/spreadsheet/src/format/locale.rs:82`, `extensions/spreadsheet/src/read/mod.rs:827`
- [ ] Bytes that are neither a workbook nor text throw `ParseError`. `extensions/spreadsheet/nvs/Reader.nvs:34`, `extensions/spreadsheet/nvs/Reader.nvs:377`, `extensions/spreadsheet/src/read/mod.rs:497`
- [ ] The tests and cases the `reader-bounds` and `reader-bounds-runs` checks name. `crates/nvs-ext/tests/spreadsheet_read.rs:1`, `tests/conformance/novis/spreadsheet-bytes-that-are-not-a-spreadsheet-throw.nvst:1`, `tests/conformance/novis/spreadsheet-an-unknown-locale-throws.nvst:1`
- [ ] `Reader::open`'s `about.md` names what throws. The fragment states the source check and its cost. The module doc's engine-exports, decompression and non-spreadsheet sentences go. `docs/examples/novis/Spreadsheet-Reader/open/about.md:1`, `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:54`, `extensions/spreadsheet/src/lib.rs:67`, `extensions/spreadsheet/src/lib.rs:90`

### 3. rows, sheets, merges and notes read what the file has

**Does:** a row of formulas reads back, `sheets()` gives the real size once per reader, its names
are accepted back, and merged fills and notes carry their text.

- [ ] `rule:core-classes/spreadsheet-reads-as-a-stream`: a row of formula cells with no cached value is kept. `extensions/spreadsheet/src/read/mod.rs:884`, `extensions/spreadsheet/src/read/mod.rs:986`
- [ ] `sheets()` counts a sheet with no dimension from its rows, once per reader, and keeps the facts. `extensions/spreadsheet/src/read/mod.rs:711`, `extensions/spreadsheet/src/read/sheet.rs:81`, `extensions/spreadsheet/src/read/binary.rs:416`
- [ ] Every sheet parameter is `tainted string|int`. `extensions/spreadsheet/nvs/Reader.nvs:71`, `extensions/spreadsheet/nvs/Reader.nvs:433`
- [ ] A merged fill copies `text`, and `notes` decodes its escapes. `extensions/spreadsheet/src/read/merge.rs:146`, `extensions/spreadsheet/src/read/extras.rs:229`, `extensions/spreadsheet/src/read/xml.rs:202`
- [ ] Limit: ids 14 and 22 stay as Excel shows them in en-US. A test pins them, and the fragment says so beside `locale`. `extensions/spreadsheet/src/format/mod.rs:78`, `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:54`
- [ ] The tests and cases the `reader-reads-what-the-file-has` checks name. `crates/nvs-ext/tests/spreadsheet_read.rs:1`, `tests/conformance/novis/spreadsheet-reads-back-a-totals-row-of-formulas.nvst:1`, `tests/conformance/novis/spreadsheet-a-sheet-name-from-sheets-reads-its-rows.nvst:1`
- [ ] The `about.md` of `rows`, `sheets` and `notes` state the change. The module doc's number formats, merges, rows, sheets and notes sentences go. `docs/examples/novis/Spreadsheet-Reader/sheets/about.md:1`, `extensions/spreadsheet/src/lib.rs:53`, `extensions/spreadsheet/src/lib.rs:60`

### 4. sheet-end parts, addresses and values are checked once, the spreadsheet list is gone, and its size is recorded

**Does:** the writer's remaining checks land, the spreadsheet list goes, and the component's size is
re-recorded.

**Split:** the Novis-half items (`Sheet.nvs`, `Workbook.nvs`, `Export.nvs` and their cases) and the
Rust items (`drawing.rs`, `tail.rs`, `writer.rs` and their tests) touch disjoint files. Two writing
subagents take one half each, side by side, and neither builds. The session builds once both end.

- [ ] `rule:core-classes/spreadsheet-sheet-end-parts`: a PNG is checked chunk by chunk to `IEND`, and an image or chart ending past the grid throws. `extensions/spreadsheet/src/drawing.rs:95`, `extensions/spreadsheet/src/drawing.rs:309`, `extensions/spreadsheet/nvs/Sheet.nvs:243`
- [ ] A `whole` rule writes no text limits, and a second `print` throws. `extensions/spreadsheet/src/tail.rs:612`, `extensions/spreadsheet/src/tail.rs:717`, `extensions/spreadsheet/nvs/Sheet.nvs:278`
- [ ] `rule:core-classes/spreadsheet-roster`: a key before the first keyed row, `column` before the first data row, and `cell` on a finished sheet. `extensions/spreadsheet/nvs/Workbook.nvs:421`, `extensions/spreadsheet/nvs/Workbook.nvs:324`, `extensions/spreadsheet/nvs/Sheet.nvs:74`, `extensions/spreadsheet/nvs/Sheet.nvs:84`
- [ ] The `writerCell` and `writerColumn` exports check the column before the cast, and `writerColumn` throws with no data row. `extensions/spreadsheet/src/lib.rs:1144`, `extensions/spreadsheet/src/lib.rs:1154`
- [ ] `rule:core-classes/spreadsheet-styles-and-defaults`: a header with `autofilter: true` writes its filter name. `extensions/spreadsheet/src/writer.rs:1064`, `extensions/spreadsheet/src/writer.rs:1103`
- [ ] `rule:core-classes/spreadsheet-value-types`: NaN and the infinities throw on the CSV path. `extensions/spreadsheet/nvs/Workbook.nvs:603`, `extensions/spreadsheet/src/cells.rs:151`
- [ ] `rule:core-classes/spreadsheet-writes-as-a-stream`: an `Export` of an `Iterator` writes once. `extensions/spreadsheet/nvs/Workbook.nvs:94`, `extensions/spreadsheet/nvs/Export.nvs:67`
- [ ] The tests and cases the `sheet-end-parts`, `writer-values` and `addresses-and-values-runs` checks name. `extensions/spreadsheet/tests/sheet_end.rs:1`, `extensions/spreadsheet/tests/write.rs:1`, `tests/conformance/novis/spreadsheet-a-key-before-the-first-keyed-row-throws.nvst:1`
- [ ] The attacks the `spreadsheet-attacks` check runs catch each new error and pass. `tests/hostile/novis/Spreadsheet-Export/bytes/01-huge-cells-and-a-used-generator.nvs:1`, `tests/hostile/novis/Spreadsheet-Sheet/image/01-files-that-pretend-to-be-images.nvs:1`
- [ ] The fragments and `about.md` files state each change. The list's last bullets and its bold lead go, and `Cost:` stays as a paragraph. Delete `data/gaps/nvs-spreadsheet/spreadsheet-reader-and-writer-leftovers.json`. `extensions/spreadsheet/src/lib.rs:38`, `extensions/spreadsheet/src/lib.rs:93`
- [ ] `cargo test --test builtin the_recorded_growth_matches_the_embedded_components` runs once, and the size it names goes into the module doc. `extensions/spreadsheet/src/lib.rs:33`, `crates/nvs-ext/tests/builtin.rs:163`

### 5. a render inside a header keeps its own first pass, and nesting and decoding stop at named bounds

**Does:** two renders in one request never share a kept body, and deep nesting, deep XML and large
decoded streams throw a named error.

- [ ] Gate: `bun nv owners --check` exits 0, and `bun nv owners --closes extension-leftovers` lists the two PDF records. `tools/nv/cmd/owners.ts:1`
- [ ] `rule:core-classes/pdf-header-is-a-page-function`: the `kept` resource replaces `KEPT`, and `Document.nvs` passes it from `first` to `second`. `extensions/pdf/src/lib.rs:430`, `extensions/pdf/src/lib.rs:526`, `wit/pdf.wit:83`, `extensions/pdf/nvs/Document.nvs:97`, `extensions/pdf/nvs/Document.nvs:127`
- [ ] The conformance case the `pdf-nested-render-runs` check names, and the header attack's step 3 renders both documents. `tests/conformance/novis/pdf-a-header-may-render-another-document.nvst:1`, `tests/hostile/novis/Pdf-Document/header/01-headers-that-throw-load-and-render-again.nvs:32`
- [ ] `rule:core-classes/pdf-one-engine`: the layout depth bound, measured, with its error. `extensions/pdf/src/layout/text.rs:505`, `extensions/pdf/src/layout/table.rs:265`, `extensions/pdf/src/parse.rs:27`
- [ ] `rule:core-classes/pdf-factur-x-is-the-one-attachment`: invoice XML past 256 levels throws. `extensions/pdf/src/factur_x.rs:83`
- [ ] `rule:core-classes/decompression-bound`: every decoded stream and attachment in `read` and `merge` runs under the bound. `extensions/pdf/src/read.rs:173`, `extensions/pdf/src/read.rs:110`, `extensions/pdf/src/merge.rs:35`
- [ ] The tests the `pdf-nesting-and-decoding-bounds` and `pdf-decoding-bounds` checks name. The bomb, nesting and XML attacks catch the new errors. `extensions/pdf/tests/render.rs:1`, `extensions/pdf/tests/read.rs:1`, `tests/hostile/novis/Pdf-Reader/open/01-a-pdf-bomb.nvs:4`, `tests/hostile/novis/Pdf-Document/invoiceXml/01-a-billion-laughs.nvs:5`, `tests/hostile/novis/Pdf-Document/fromHtml/01-html-nested-and-broken.nvs:5`
- [ ] Limits: the two-million-character word, the million `q` operators and, if still at the limit, the nested `div`s are stated in the fragment with the memory limit's size. Their attacks keep `ends-early`. The module doc's first two bullets go. `docs/rules/core-classes/pdf-one-engine.md:1`, `tests/hostile/novis/Pdf-Document/fromHtml/02-one-word-of-two-million-characters.nvs:4`, `tests/hostile/novis/Pdf-Reader/text/02-a-page-that-saves-its-state.nvs:4`, `extensions/pdf/src/lib.rs:47`

### 6. a reader keeps its file in the engine, page strings match as written, and a second reader checks the passwords

**Does:** `Reader::text` sends no file, string names are case-sensitive, `merge`'s cost is measured,
and encryption and the standards are checked by tools other than ours.

**Split:** the veraPDF item (`tools/nv/cmd/pdf-validate.ts`, `.github/workflows/ci.yml`) shares no
file with the rest. A writing subagent takes it, side by side with the session, and builds nothing.

- [ ] `rule:core-classes/pdf-reader-and-merge`: the `reading` resource, and `text` takes it. `wit/pdf.wit:166`, `extensions/pdf/nvs/Reader.nvs:21`, `extensions/pdf/nvs/Reader.nvs:42`, `extensions/pdf/src/lib.rs:430`
- [ ] `rule:core-classes/pdf-header-is-a-page-function`: string names are case-sensitive on both sides. `extensions/pdf/nvs/Page.nvs:36`, `extensions/pdf/src/layout/margin.rs:63`, `extensions/pdf/src/paginate/mod.rs:286`
- [ ] `merge` measured once on its bench. The cause is fixed if one step holds more than half the time, and the rest is a `Cost:` sentence. `extensions/pdf/src/merge.rs:35`, `benches/members/novis/Pdf-Pages/merge.nvs:1`
- [ ] `rule:core-classes/pdf-reader-and-merge`: predefined CMaps, or the limit stated. `extensions/pdf/src/read.rs:237`, `extensions/pdf/src/read.rs:24`, `extensions/pdf/Cargo.toml:20`
- [ ] `rule:core-classes/pdf-standards-and-passwords`: `lopdf` as a dev-dependency, and the tests the `pdf-second-reader` check names. `crates/nvs-ext/Cargo.toml:1`, `crates/nvs-ext/tests/pdf_standards.rs:1`
- [ ] `bun nv pdf-validate` and CI's Linux job that runs it. `tools/nv/cmd/pdf-validate.ts:1`, `.github/workflows/ci.yml:1`
- [ ] The tests and the case the `pdf-reader-and-strings` and `pdf-page-string-runs` checks name. `extensions/pdf/tests/read.rs:1`, `tests/conformance/novis/pdf-page-string-matches-the-name-as-written.nvst:1`
- [ ] The `about.md` of `Reader::text` and `Page::string` state the change. The fragments state the reading, the case rule and the checks. The module doc's merge and passwords bullets go. `docs/examples/novis/Pdf-Reader/text/about.md:1`, `docs/rules/core-classes/pdf-standards-and-passwords.md:25`, `extensions/pdf/src/lib.rs:54`, `extensions/pdf/src/lib.rs:76`

### 7. a table is as wide as its columns, splits a tall rowspan, aligns its cells and draws its rows

**Does:** five of the table module's six faults are fixed. The sixth, a mark in a cell, is stage 8's.

- [ ] `rule:core-classes/pdf-tables-across-pages`: a table without a `width` is as wide as its columns, up to its containing block. `extensions/pdf/src/layout/table.rs:265`, `extensions/pdf/src/layout/table.rs:303`
- [ ] A `rowspan` piece taller than a page splits between lines. `extensions/pdf/src/layout/table.rs:335`, `extensions/pdf/src/layout/table.rs:401`
- [ ] A cell's `vertical-align`, and a row's and a group's background and collapsed borders. `extensions/pdf/src/layout/table.rs:617`, `extensions/pdf/src/layout/table.rs:628`
- [ ] A `display: table` element in a row is wrapped in an anonymous cell. `extensions/pdf/src/layout/inline.rs:939`, `extensions/pdf/src/layout/inline.rs:1382`
- [ ] Each new value is in `subset.json`. `extensions/pdf/subset.json:1`
- [ ] The tests and the case the `pdf-tables` and `pdf-tables-runs` checks name. `extensions/pdf/tests/tables.rs:1`, `tests/conformance/novis/pdf-a-table-is-as-wide-as-its-content.nvst:1`
- [ ] The fragment states the five, and the module doc keeps only the mark sentence and `Cost:`. `docs/rules/core-classes/pdf-tables-across-pages.md:1`, `extensions/pdf/src/layout/table.rs:62`

### 8. a mark, a string, a running element, an id and a heading are on the page they are drawn on

**Does:** content inside a cell, an inline-block, a float or a flex item is recorded on the page it
is drawn on, and counters and `element()` read as CSS says.

- [ ] `rule:core-classes/pdf-css-subset`: a `data-mark`, `string-set` or running element inside a cell, an inline-block, a float or a flex item is recorded on its drawn page. `extensions/pdf/src/layout/inline.rs:1588`, `extensions/pdf/src/layout/inline.rs:1599`, `extensions/pdf/src/layout/table.rs:662`
- [ ] An `id` there has its page, and so does an `id` in a running element. `extensions/pdf/src/layout/block.rs:363`, `extensions/pdf/src/layout/block.rs:381`
- [ ] `counter()` in a margin box and `target-counter()` read any counter. `extensions/pdf/src/paginate/mod.rs:349`, `extensions/pdf/src/layout/inline.rs:448`
- [ ] `element()` reads `start` and `first-except`. `extensions/pdf/src/paginate/mod.rs:376`, `extensions/pdf/src/layout/margin.rs:124`
- [ ] `rule:core-classes/pdf-standards-and-passwords`: a heading there has its bookmark. `extensions/pdf/src/layout/block.rs:387`, `extensions/pdf/src/write.rs:382`
- [ ] The tests the `pdf-page-attribution` and `pdf-bookmarks` checks name. `extensions/pdf/tests/paginate.rs:1`, `extensions/pdf/tests/render.rs:1`
- [ ] The marks attack still passes. The table module's list goes, with its bold lead, and `Cost:` stays. Delete `data/gaps/nvs-pdf/pdf-table-leftovers.json`. The pagination bullet loses these sentences. `tests/hostile/novis/Pdf-Page/marks/01-hostile-marks.nvs:1`, `extensions/pdf/src/layout/table.rs:62`, `extensions/pdf/src/lib.rs:57`

### 9. page breaks keep what avoid asks for, and a left or right page sets its own line widths

**Does:** the break properties do what the subset says, and a float in a paragraph is placed at its
line.

- [ ] `rule:core-classes/pdf-css-subset`: a `:left` or `:right` page of another width sets its own line widths. `extensions/pdf/src/paginate/mod.rs:30`, `extensions/pdf/src/layout/text.rs:85`, `extensions/pdf/src/layout/inline.rs:1288`
- [ ] `break-inside: avoid` keeps a row, a flex container and an inline-block whole. `extensions/pdf/src/layout/block.rs:218`, `extensions/pdf/src/layout/text.rs:509`
- [ ] `break-after: avoid` and `break-before: avoid`. `extensions/pdf/src/layout/block.rs:88`, `extensions/pdf/src/layout/block.rs:473`, `extensions/pdf/src/layout/block.rs:487`
- [ ] A float in a paragraph is placed at its line, and goes on to the next page. `extensions/pdf/src/layout/block.rs:604`
- [ ] Each new value is in `subset.json`, and the tests the `pdf-breaks` check names. `extensions/pdf/subset.json:1`, `extensions/pdf/tests/paginate.rs:1`
- [ ] The module doc's pagination bullet goes. `extensions/pdf/src/lib.rs:57`

### 10. absolute and transformed boxes, the grid shorthands, elliptical radii and repeating gradients

**Does:** the layout bullets for boxes and paint are fixed, and a link follows its transformed box.

**Split:** the paint items (`layout/boxes.rs`) and the grid item (`layout/grid.rs` and the CSS
shorthand parse) share no file with the block items. Two writing subagents take them, side by side
with the session, and none builds.

- [ ] `rule:core-classes/pdf-css-subset`: an absolute box with `left` and `right` and no `width` stretches. `extensions/pdf/src/layout/block.rs:168`, `extensions/pdf/src/layout/block.rs:202`
- [ ] A transformed box is a containing block. `extensions/pdf/src/layout/block.rs:912`, `extensions/pdf/src/layout/block.rs:945`
- [ ] `rule:core-classes/pdf-output-is-inert`: a link in a transformed box covers the box as drawn. `extensions/pdf/src/write.rs:628`, `extensions/pdf/src/write.rs:570`
- [ ] The `grid-template` and `grid` shorthands are read. `extensions/pdf/src/layout/grid.rs:34`
- [ ] Elliptical radii and repeating gradients are drawn. `extensions/pdf/src/layout/boxes.rs:568`, `extensions/pdf/src/layout/boxes.rs:336`
- [ ] Each new value is in `subset.json`, and the tests the `pdf-boxes` and `pdf-transformed-link` checks name. `extensions/pdf/subset.json:1`, `extensions/pdf/tests/layout.rs:1`, `extensions/pdf/tests/render.rs:1`
- [ ] The module doc's writing bullet goes, and the layout bullet keeps only its flex sentences. `extensions/pdf/src/lib.rs:65`, `extensions/pdf/src/lib.rs:67`

### 11. flex items, right-to-left and SVG text are laid out as HTML's, clippy is clean, and the PDF lists are gone

**Does:** the last layout and text bullets are fixed, clippy has no warning, the PDF list goes, and
the component's size is re-recorded.

- [ ] `rule:core-classes/pdf-css-subset`: `::before`, `::after` and `<br>` are flex items. `extensions/pdf/src/layout/inline.rs:1127`, `extensions/pdf/src/layout/inline.rs:1487`, `extensions/pdf/src/layout/inline.rs:1524`
- [ ] Baseline alignment and `margin: auto` on a flex item. `extensions/pdf/src/layout/flex.rs:163`
- [ ] `hyphenate-character` is in `subset.json`, and an inline box in a right-to-left line is placed right to left. `extensions/pdf/subset.json:1`, `extensions/pdf/src/layout/inline.rs:114`, `extensions/pdf/src/layout/text.rs:795`
- [ ] `rule:core-classes/pdf-default-fonts`: an SVG `font-family` maps as HTML's does. `extensions/pdf/src/fonts.rs:119`, `extensions/pdf/src/fonts.rs:45`
- [ ] The tests the `pdf-flex-and-text` and `pdf-svg-fonts` checks name. `extensions/pdf/tests/layout.rs:1`, `extensions/pdf/tests/render.rs:1`
- [ ] `rule:packaging/the-first-party-components-are-built-in`: the clippy lints are fixed, and the check runs once. `extensions/pdf/src/layout.rs:459`, `extensions/pdf/src/layout/inline.rs:1242`, `extensions/pdf/src/css/value.rs:1`, `extensions/pdf/src/css/mod.rs:1`
- [ ] The list goes, with its bold lead. Delete `data/gaps/nvs-pdf/pdf-leftovers.json`. The size is re-recorded once. `extensions/pdf/src/lib.rs:45`, `extensions/pdf/src/lib.rs:39`, `crates/nvs-ext/tests/builtin.rs:163`
- [ ] The attacks the `pdf-attacks` check runs pass, `bun nv rules --render` runs, and `bun nv owners --closes extension-leftovers` passes. `tools/nv/cmd/owners.ts:1`

### 12. the goal is verified once, and every failure is fixed

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

Stage 1 is done and committed. `extensions/spreadsheet/src/reference.rs` is the one reference
parser: `Workbook::define` (xlsx and ods) and a chart's series range read through it, and
`check_name_sheets` checks each name's sheet at `finish()`. `check_sheet_name` refuses control
characters and `History`. `cells::check_formula` (1 to 8,192 characters) holds cells, table totals,
validations and the `Cell` and `Formula` conditional formats, and `Formula::of('')` throws in
`Formula.nvs`. Chart titles and axis titles stop at 255. The checks `names-and-references`,
`chart-ranges`, `conditional-formula-bound` and `names-and-references-runs` ran green once by name.
The `Reader::names` example `01` now defines a range in place of the constant `0.2`, which
`define` declines. The sweep's nightly reds (run `41187571578` on `857b39cad`) are fixed by later
commits: `bfce9c2d2` (shots), `9fe05ce88` (ASAN ODR), `8ce92dda5` (Miri sizing) and `39795c9c4`
(browser `NVS_BIN`). `env-secret-proofs` has no definition in the tree.

### Next

Stage 2, over `extensions/spreadsheet/src/read/mod.rs`, `read/zip.rs`, `format/locale.rs`,
`src/lib.rs`'s reader exports and `nvs/Reader.nvs`. Its gate is stage 1's, already green.

- [ ] `read::Reader` keeps the source's size and an FNV-1a hash of its first and last 512 bytes, and every call checks them first. `extensions/spreadsheet/src/read/mod.rs:484`, `extensions/spreadsheet/src/lib.rs:1163`
- [ ] `readerRows` with `max` 0 throws, and `readerProperties` reads once per reader. `extensions/spreadsheet/src/lib.rs:1200`, `extensions/spreadsheet/src/lib.rs:1339`, `extensions/spreadsheet/src/read/mod.rs:793`
- [ ] The two decompression-bound messages name no rule, and say which option raises the bound. `extensions/spreadsheet/src/read/zip.rs:79`, `extensions/spreadsheet/src/read/zip.rs:86`
- [ ] An unknown `locale` throws `LogicError` naming it. `extensions/spreadsheet/src/format/locale.rs:82`, `extensions/spreadsheet/src/read/mod.rs:827`
- [ ] Bytes that are neither a workbook nor text throw `ParseError`. `extensions/spreadsheet/nvs/Reader.nvs:34`, `extensions/spreadsheet/nvs/Reader.nvs:377`, `extensions/spreadsheet/src/read/mod.rs:497`
- [ ] The tests, cases, `about.md` and fragment sentences stage 2 lists. `crates/nvs-ext/tests/spreadsheet_read.rs:1`, `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:54`

The line numbers above were read before stage 1 and may have moved a few lines in `src/lib.rs`.
Expect red at the gate: any test outside this component that defines a name with a constant or
formula value, writes a sheet named `History`, or writes an empty `Formula`. None was found in
`crates/nvs-ext/tests/`, the conformance tree or the examples.

### Backlog

- The module doc bullets before `extensions/spreadsheet/src/lib.rs:53`, and the ods and extras bullets: owned by `spreadsheet-second-wave` (`docs/goals/spreadsheet-second-wave.md`).
- A clippy run over the component crates inside `bun nv verify`: declined here, see § *Clippy* (`tools/nv/cmd/verify.ts`).
