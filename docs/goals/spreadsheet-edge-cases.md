---
looper: 1
title: "the spreadsheet engine handles every failure class PhpSpreadsheet, OpenSpout and fast-excel-writer got wrong"
summary: "every failure class from the three libraries' trackers, changelogs and advisories is fixed or pinned by a green test, and each declined one is recorded in its rule's Declined list"
context: [docs/rules/core-classes/spreadsheet-reads-as-a-stream.md, docs/rules/core-classes/spreadsheet-value-types.md]
stages:
  - id: 1
    title: "hostile input reads in linear time and memory, an encrypted file throws, a filled template keeps no macro part, and a part in another encoding throws"
    status: open
    context: [extensions/spreadsheet/src/read/ods.rs, extensions/spreadsheet/src/read/xml.rs, extensions/spreadsheet/src/read/mod.rs, extensions/spreadsheet/src/read/xls.rs, extensions/spreadsheet/src/template/mod.rs, crates/nvs-ext/tests/spreadsheet_hostile.rs, tests/hostile/README.md]
  - id: 2
    title: "xlsx reading resolves odd part names, reads empty and date cells, joins surrogate pairs, and lists a sheet whose part is missing"
    status: open
    context: [extensions/spreadsheet/src/read/mod.rs, extensions/spreadsheet/src/read/zip.rs, extensions/spreadsheet/src/read/xml.rs, extensions/spreadsheet/src/read/extras.rs, extensions/spreadsheet/src/read/merge.rs, crates/nvs-ext/tests/spreadsheet_read.rs]
  - id: 3
    title: "xls reads continued text and code pages, ods reads offsets, void cells and whitespace as ODF says, and a CSV sep= line sets the separator"
    status: open
    context: [extensions/spreadsheet/src/read/xls.rs, extensions/spreadsheet/src/read/xls_extras.rs, extensions/spreadsheet/src/read/ods.rs, extensions/spreadsheet/nvs/Reader.nvs, crates/nvs-ext/tests/spreadsheet_read.rs]
  - id: 4
    title: "the writer prefixes new functions, keeps line breaks in attributes, counts lengths as Excel does, stores an image once, and a CSV export opens cleanly in Excel"
    status: open
    context: [extensions/spreadsheet/src/cells.rs, extensions/spreadsheet/src/xml.rs, extensions/spreadsheet/src/writer.rs, extensions/spreadsheet/src/table.rs, extensions/spreadsheet/src/tail.rs, extensions/spreadsheet/nvs/Workbook.nvs, crates/nvs-stdlib/src/csv.rs, extensions/spreadsheet/tests/write.rs]
  - id: 5
    title: "a template fill moves every part that points at a row, and the evaluator reads escaped table columns, 1904 date text and locale text"
    status: open
    context: [extensions/spreadsheet/src/template/mod.rs, extensions/spreadsheet/src/template/shift.rs, extensions/spreadsheet/src/eval/parse.rs, extensions/spreadsheet/src/eval/value.rs, extensions/spreadsheet/src/eval/mod.rs, extensions/spreadsheet/tests/template.rs, extensions/spreadsheet/tests/evaluate.rs, docs/rules/core-classes/spreadsheet-template-fill.md]
  - id: 6
    title: "the hostile and reading classes the engine already handles are pinned by tests"
    status: open
    context: [crates/nvs-ext/tests/spreadsheet_hostile.rs, crates/nvs-ext/tests/spreadsheet_read.rs, tests/hostile/README.md]
  - id: 7
    title: "the date, number, format, formula and writing classes are pinned by tests, and every declined class is in its rule's Declined list"
    status: open
    context: [extensions/spreadsheet/tests/format.rs, extensions/spreadsheet/tests/evaluate.rs, extensions/spreadsheet/tests/template.rs, extensions/spreadsheet/tests/write.rs, extensions/spreadsheet/tests/sheet_end.rs, crates/nvs-ext/tests/spreadsheet_read.rs]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: hostile-input
    stage: 1
    name: repeated ods rows, long tokens, shared strings, formula groups and format lookups cost linear work, encrypted files and foreign encodings throw, and a filled template has no macro part
    run: [cargo, test, --test, spreadsheet_hostile, --, an_ods_row_repeated_to_the_last_row_costs_one_row_to_read, one_token_longer_than_many_read_blocks_is_scanned_once, the_shared_string_table_counts_against_the_archive_ceiling, many_shared_formula_groups_in_a_row_read_in_linear_time, many_custom_number_formats_read_in_linear_time, a_long_format_code_with_many_g_and_a_letters_renders_in_linear_time, a_whole_column_reference_visits_only_that_column, an_encrypted_xlsx_and_an_xls_with_a_password_throw_that_encryption_is_not_supported, a_filled_template_drops_macro_sheets_activex_ribbons_and_ole_objects_and_keeps_images, a_part_in_utf16_or_another_declared_encoding_throws_naming_the_part]
    timeout: 1800
    tests:
      - an_ods_row_repeated_to_the_last_row_costs_one_row_to_read
      - one_token_longer_than_many_read_blocks_is_scanned_once
      - the_shared_string_table_counts_against_the_archive_ceiling
      - many_shared_formula_groups_in_a_row_read_in_linear_time
      - many_custom_number_formats_read_in_linear_time
      - a_long_format_code_with_many_g_and_a_letters_renders_in_linear_time
      - a_whole_column_reference_visits_only_that_column
      - an_encrypted_xlsx_and_an_xls_with_a_password_throw_that_encryption_is_not_supported
      - a_filled_template_drops_macro_sheets_activex_ribbons_and_ole_objects_and_keeps_images
      - a_part_in_utf16_or_another_declared_encoding_throws_naming_the_part
  - id: csv-charset
    stage: 1
    name: a UTF-32 CSV throws, and a CSV that is not UTF-8 names the encoding option
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-a-utf32-csv-throws-and-a-csv-that-is-not-utf8-names-the-encoding-option.nvst]
    timeout: 1800
    tests:
      - spreadsheet-a-utf32-csv-throws-and-a-csv-that-is-not-utf8-names-the-encoding-option
  - id: xlsx-reading
    stage: 2
    name: odd part names resolve, empty and date cells read, surrogate pairs join, merges with dollars, notes without phonetics, one hash per link, a missing sheet part, serials 0 and 60, and large durations
    run: [cargo, test, --test, spreadsheet_read, --, backslashes_and_percent_escapes_in_part_names_and_targets_resolve, shared_strings_missing_from_the_workbook_rels_are_found_at_their_usual_path, an_empty_value_reads_as_null_and_a_date_cell_reads_as_its_date_type, a_number_cell_with_text_throws_naming_the_cell, an_escaped_surrogate_pair_reads_as_one_character, a_merged_range_written_with_dollar_signs_reads_as_the_range, phonetic_text_in_a_note_is_not_part_of_the_note, an_external_link_with_a_fragment_and_a_location_has_one_hash, a_sheet_whose_part_is_missing_is_listed_with_no_size_and_throws_when_read, serials_zero_and_sixty_read_as_plain_numbers, a_duration_serial_past_the_range_of_a_time_reads_as_its_number]
    timeout: 1800
    tests:
      - backslashes_and_percent_escapes_in_part_names_and_targets_resolve
      - shared_strings_missing_from_the_workbook_rels_are_found_at_their_usual_path
      - an_empty_value_reads_as_null_and_a_date_cell_reads_as_its_date_type
      - a_number_cell_with_text_throws_naming_the_cell
      - an_escaped_surrogate_pair_reads_as_one_character
      - a_merged_range_written_with_dollar_signs_reads_as_the_range
      - phonetic_text_in_a_note_is_not_part_of_the_note
      - an_external_link_with_a_fragment_and_a_location_has_one_hash
      - a_sheet_whose_part_is_missing_is_listed_with_no_size_and_throws_when_read
      - serials_zero_and_sixty_read_as_plain_numbers
      - a_duration_serial_past_the_range_of_a_time_reads_as_its_number
  - id: xls-ods-reading
    stage: 3
    name: continued xls text, property code pages, ods offsets, void cells, string values and ODF whitespace read as written
    run: [cargo, test, --test, spreadsheet_read, --, an_xls_text_cell_continued_across_records_reads_whole, xls_properties_decode_by_their_code_page, an_ods_date_with_an_offset_reads_as_the_wall_clock_in_the_zone, an_ods_void_cell_reads_as_null_and_a_string_value_is_its_text, pretty_printed_ods_cells_read_without_the_layout_whitespace, an_empty_ods_paragraph_is_a_line_of_the_cell]
    timeout: 1800
    tests:
      - an_xls_text_cell_continued_across_records_reads_whole
      - xls_properties_decode_by_their_code_page
      - an_ods_date_with_an_offset_reads_as_the_wall_clock_in_the_zone
      - an_ods_void_cell_reads_as_null_and_a_string_value_is_its_text
      - pretty_printed_ods_cells_read_without_the_layout_whitespace
      - an_empty_ods_paragraph_is_a_line_of_the_cell
  - id: csv-separator-line
    stage: 3
    name: a first line sep=; sets the separator and is not a row
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-a-csv-that-starts-with-sep-uses-that-separator.nvst]
    timeout: 1800
    tests:
      - spreadsheet-a-csv-that-starts-with-sep-uses-that-separator
  - id: writing
    stage: 4
    name: new functions get their prefix, attribute line breaks survive, lengths count UTF-16 units, numeric table headers are text, whole-column validations are written, and an image is stored once
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, write, --, a_function_newer_than_excel_2007_is_written_with_its_prefix, a_function_name_inside_a_string_is_not_prefixed, line_breaks_and_tabs_in_attribute_values_are_written_as_character_references, text_and_sheet_name_lengths_count_utf16_units, sheet_names_that_differ_only_in_case_are_the_same_name_by_upper_case_folding, a_numeric_table_header_is_written_as_text, a_validation_on_whole_columns_or_several_ranges_is_written_and_a_quote_in_a_list_value_is_doubled, the_same_image_added_twice_is_stored_once]
    timeout: 1800
    tests:
      - a_function_newer_than_excel_2007_is_written_with_its_prefix
      - a_function_name_inside_a_string_is_not_prefixed
      - line_breaks_and_tabs_in_attribute_values_are_written_as_character_references
      - text_and_sheet_name_lengths_count_utf16_units
      - sheet_names_that_differ_only_in_case_are_the_same_name_by_upper_case_folding
      - a_numeric_table_header_is_written_as_text
      - a_validation_on_whole_columns_or_several_ranges_is_written_and_a_quote_in_a_list_value_is_doubled
      - the_same_image_added_twice_is_stored_once
  - id: csv-and-file-export
    stage: 4
    name: a failed write leaves no file, a CSV export starts with a BOM and ends records with CRLF, and Core\Csv quotes edge spaces and takes a line end
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-a-failed-write-to-a-file-leaves-no-file.nvst, --case, tests/conformance/novis/spreadsheet-a-csv-export-starts-with-a-bom-and-ends-records-with-crlf.nvst, --case, tests/conformance/core/csv-format-quotes-the-separator-the-quote-cr-lf-and-edge-spaces.nvst, --case, tests/conformance/core/csv-format-ends-each-record-with-the-line-end-option.nvst]
    timeout: 1800
    tests:
      - spreadsheet-a-failed-write-to-a-file-leaves-no-file
      - spreadsheet-a-csv-export-starts-with-a-bom-and-ends-records-with-crlf
      - csv-format-quotes-the-separator-the-quote-cr-lf-and-edge-spaces
      - csv-format-ends-each-record-with-the-line-end-option
  - id: formulas-and-fill
    stage: 5
    name: a fill moves notes, breaks, link locations, threaded comments and pivots, un-shares a crossing group, recalculates, leaves external and keeps lower-case references right; the evaluator reads hash columns, 1904 text and locale text
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, template, --test, evaluate, --, notes_page_breaks_and_link_locations_move_with_their_rows, threaded_comments_and_pivot_references_move_with_their_rows, a_shared_formula_group_across_the_repeated_block_is_written_unshared, a_filled_template_always_recalculates_on_load, an_external_reference_is_never_shifted, a_lower_case_reference_in_a_template_formula_is_shifted, a_table_column_named_with_a_hash_is_found_by_its_escaped_name, a_date_text_in_a_1904_workbook_counts_from_1904, text_that_needs_a_locale_to_be_a_number_is_listed_and_keeps_its_stored_value]
    timeout: 1800
    tests:
      - notes_page_breaks_and_link_locations_move_with_their_rows
      - threaded_comments_and_pivot_references_move_with_their_rows
      - a_shared_formula_group_across_the_repeated_block_is_written_unshared
      - a_filled_template_always_recalculates_on_load
      - an_external_reference_is_never_shifted
      - a_lower_case_reference_in_a_template_formula_is_shifted
      - a_table_column_named_with_a_hash_is_found_by_its_escaped_name
      - a_date_text_in_a_1904_workbook_counts_from_1904
      - text_that_needs_a_locale_to_be_a_number_is_listed_and_keeps_its_stored_value
  - id: pinned-hostile
    stage: 6
    name: every hostile class the engine already handles has a test
    run: [cargo, test, --test, spreadsheet_hostile, --, an_external_entity_in_any_part_and_any_encoding_reads_nothing, an_entity_in_an_ods_part_is_never_expanded, a_bomb_in_a_rels_part_or_ods_content_ends_at_the_bound, a_cfb_with_a_difat_cycle_a_mini_fat_cycle_or_a_short_stream_throws, a_target_outside_the_package_is_never_read, a_relationship_cycle_or_self_reference_is_read_once, a_reference_chain_of_a_hundred_thousand_cells_evaluates_or_throws_cleanly]
    timeout: 1800
    tests:
      - an_external_entity_in_any_part_and_any_encoding_reads_nothing
      - an_entity_in_an_ods_part_is_never_expanded
      - a_bomb_in_a_rels_part_or_ods_content_ends_at_the_bound
      - a_cfb_with_a_difat_cycle_a_mini_fat_cycle_or_a_short_stream_throws
      - a_target_outside_the_package_is_never_read
      - a_relationship_cycle_or_self_reference_is_read_once
      - a_reference_chain_of_a_hundred_thousand_cells_evaluates_or_throws_cleanly
  - id: pinned-reading
    stage: 6
    name: every xlsx, xlsb, xls and ods reading class the engine already handles has a test
    run: [cargo, test, --test, spreadsheet_read, --, prefixed_and_strict_namespaces_read_as_the_transitional_ones, cells_and_rows_without_a_reference_take_the_next_place, a_missing_or_wrong_dimension_does_not_limit_the_rows, an_empty_shared_string_phonetic_text_and_a_missing_count_read_as_written, an_inline_string_of_several_runs_reads_as_one_text, a_shared_formula_whose_master_is_outside_the_read_rows_reads_its_formula, flags_written_as_numbers_or_words_read_alike, sheets_are_listed_in_workbook_order_not_part_order, bytes_before_the_zip_throw_that_it_is_not_a_spreadsheet, a_truncated_xlsb_record_throws, xls_formulas_with_an_array_constant_a_quote_and_a_deleted_sheet_read, an_rk_value_of_0_07_reads_exactly, an_ods_without_a_mimetype_reads, an_ods_sheet_name_with_a_dot_reads, an_ods_number_style_with_three_leading_zeros_formats_as_written]
    timeout: 1800
    tests:
      - prefixed_and_strict_namespaces_read_as_the_transitional_ones
      - cells_and_rows_without_a_reference_take_the_next_place
      - a_missing_or_wrong_dimension_does_not_limit_the_rows
      - an_empty_shared_string_phonetic_text_and_a_missing_count_read_as_written
      - an_inline_string_of_several_runs_reads_as_one_text
      - a_shared_formula_whose_master_is_outside_the_read_rows_reads_its_formula
      - flags_written_as_numbers_or_words_read_alike
      - sheets_are_listed_in_workbook_order_not_part_order
      - bytes_before_the_zip_throw_that_it_is_not_a_spreadsheet
      - a_truncated_xlsb_record_throws
      - xls_formulas_with_an_array_constant_a_quote_and_a_deleted_sheet_read
      - an_rk_value_of_0_07_reads_exactly
      - an_ods_without_a_mimetype_reads
      - an_ods_sheet_name_with_a_dot_reads
      - an_ods_number_style_with_three_leading_zeros_formats_as_written
  - id: pinned-csv
    stage: 6
    name: every CSV reading and export class the engine already handles has a case
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/spreadsheet-a-csv-export-prefixes-an-apostrophe-to-text-that-starts-a-formula.nvst, --case, tests/conformance/novis/spreadsheet-reads-csv-quotes-backslashes-and-a-bare-cr-as-written.nvst, --case, tests/conformance/novis/spreadsheet-csv-values-that-look-like-numbers-stay-strings.nvst, --case, tests/conformance/novis/spreadsheet-a-csv-field-of-32767-characters-reads-whole.nvst]
    timeout: 1800
    tests:
      - spreadsheet-a-csv-export-prefixes-an-apostrophe-to-text-that-starts-a-formula
      - spreadsheet-reads-csv-quotes-backslashes-and-a-bare-cr-as-written
      - spreadsheet-csv-values-that-look-like-numbers-stay-strings
      - spreadsheet-a-csv-field-of-32767-characters-reads-whole
  - id: pinned-dates
    stage: 7
    name: every date and number reading class the engine already handles has a test
    run: [cargo, test, --test, spreadsheet_read, --, dates_in_a_1904_workbook_read_as_their_days, a_date_time_in_a_daylight_saving_gap_or_overlap_reads_as_one_instant, milliseconds_in_a_serial_read_into_the_time_of_day, a_percent_cell_reads_as_its_fraction, a_cached_text_result_keeps_its_leading_zeros]
    timeout: 1800
    tests:
      - dates_in_a_1904_workbook_read_as_their_days
      - a_date_time_in_a_daylight_saving_gap_or_overlap_reads_as_one_instant
      - milliseconds_in_a_serial_read_into_the_time_of_day
      - a_percent_cell_reads_as_its_fraction
      - a_cached_text_result_keeps_its_leading_zeros
  - id: pinned-values
    stage: 7
    name: every format, formula, template and writing class the engine already handles has a test
    run: [cargo, test, --manifest-path, extensions/spreadsheet/Cargo.toml, --test, format, --test, evaluate, --test, template, --test, write, --test, sheet_end, --, color_and_general_keywords_in_any_case_and_a_condition_section_render, a_fraction_code_with_a_zero_denominator_renders_without_a_panic, a_system_long_date_code_renders_as_a_long_date, a_fifty_thousand_character_format_code_renders_in_linear_time, sheet_names_with_apostrophes_bangs_and_non_latin_letters_parse, whole_column_ranges_and_chained_ranges_parse, an_empty_argument_is_evaluated_as_excel_does, a_string_that_looks_like_a_reference_stays_a_string, names_that_look_like_cells_or_have_dots_or_non_ascii_letters_parse, implicit_intersection_and_spill_references_are_listed, chained_comparisons_evaluate_left_to_right, an_empty_string_is_not_equal_to_zero, an_error_inside_a_range_gives_that_error, ifs_switch_and_choose_evaluate_only_the_branch_taken, rounddown_and_roundup_correct_binary_noise, subtotal_skips_hidden_rows_and_nested_subtotals, relative_and_quoted_sheet_names_are_shifted, a_quoted_sheet_name_that_looks_like_a_cell_is_not_shifted_as_a_cell, media_header_images_and_note_drawings_are_copied_unchanged, cells_of_a_row_with_gaps_are_written_in_column_order, a_freeze_of_columns_only_or_both_axes_writes_its_pane, an_internal_link_to_a_quoted_sheet_name_is_written_quoted, a_note_of_several_lines_keeps_its_line_breaks, a_merge_over_an_autofilter_row_is_written, sheet_protection_has_no_lock_revision]
    timeout: 1800
    tests:
      - color_and_general_keywords_in_any_case_and_a_condition_section_render
      - a_fraction_code_with_a_zero_denominator_renders_without_a_panic
      - a_system_long_date_code_renders_as_a_long_date
      - a_fifty_thousand_character_format_code_renders_in_linear_time
      - sheet_names_with_apostrophes_bangs_and_non_latin_letters_parse
      - whole_column_ranges_and_chained_ranges_parse
      - an_empty_argument_is_evaluated_as_excel_does
      - a_string_that_looks_like_a_reference_stays_a_string
      - names_that_look_like_cells_or_have_dots_or_non_ascii_letters_parse
      - implicit_intersection_and_spill_references_are_listed
      - chained_comparisons_evaluate_left_to_right
      - an_empty_string_is_not_equal_to_zero
      - an_error_inside_a_range_gives_that_error
      - ifs_switch_and_choose_evaluate_only_the_branch_taken
      - rounddown_and_roundup_correct_binary_noise
      - subtotal_skips_hidden_rows_and_nested_subtotals
      - relative_and_quoted_sheet_names_are_shifted
      - a_quoted_sheet_name_that_looks_like_a_cell_is_not_shifted_as_a_cell
      - media_header_images_and_note_drawings_are_copied_unchanged
      - cells_of_a_row_with_gaps_are_written_in_column_order
      - a_freeze_of_columns_only_or_both_axes_writes_its_pane
      - an_internal_link_to_a_quoted_sheet_name_is_written_quoted
      - a_note_of_several_lines_keeps_its_line_breaks
      - a_merge_over_an_autofilter_row_is_written
      - sheet_protection_has_no_lock_revision
---

# the spreadsheet engine handles every failure class PhpSpreadsheet, OpenSpout and fast-excel-writer got wrong

## Why

The user asked for a review of the most used PHP spreadsheet libraries: PhpSpreadsheet, OpenSpout
with its predecessor box/spout, and fast-excel-writer with fast-excel-reader. On 2026-11-04 research
subagents read their issue trackers (about 2,900 issues), their merged pull requests, four changelogs,
about 60 security advisories and their test-fixture lists, and grouped them into failure classes:
hostile input (SEC), xlsx reading (RX), shared strings and streaming (ST), xlsb (RB), xls (RL), ods
(RO), CSV reading (RC) and writing (WC), dates (DT), numbers (NM), number formats (NF), formula
parsing (FP) and evaluation (FE), xlsx writing (WX) and template fill (TP). Triage subagents then read
`extensions/spreadsheet/`, `extensions/spreadsheet/nvs/` and `Core\Csv` against each class. This goal
is the result. It holds everything a session needs; no session re-reads the trackers.

**Where the engine already stands.** The component has no I/O (`rule:core-classes/spreadsheet-has-no-io`),
no entity is ever expanded and no DTD loaded, every part is read under the decompression bound, a
string never writes as a formula (`rule:core-classes/spreadsheet-formula-is-a-value`), a written file
has no macro (`rule:core-classes/spreadsheet-output-is-inert`), and most classes the libraries hit are
already handled. Stages 6 and 7 pin each of those with a test of the exact shape the libraries got
wrong.

**What the triage found wrong.** The largest correctness gap is that no `_xlfn.` prefix is ever
written, so every function newer than Excel 2007 opens as `#NAME?`. Beyond that: an ods row repeated
to the end of the grid is cloned into storage, a long token is rescanned on every refill, an
encrypted file reads as garbage or a wrong error, a template fill keeps XLM macro sheets and ActiveX
parts, a part in UTF-16 is lexed as garbage, a fill does not move notes, page breaks or pivots, and
several reading, writing and evaluation corners are wrong. Stages 1 to 5 fix those.

## Standing decisions

1. **No new limits.** Hostile input ends at the request's memory ceiling and CPU deadline. A quadratic path is made linear, never capped.
2. **A repeated ods row is yielded again and never cloned into storage**, so reading costs the memory of one row whatever `number-rows-repeated` says.
3. **The XML scan resumes where the last refill stopped**, so one long token is scanned once.
4. **The shared-string arena is charged to the archive-wide ceiling** that the whole-archive read (`whole`, 64 MiB) already uses.
5. **Shared-formula groups are indexed by `si`, custom formats are found by a map, and the format renderer slices without copying**; each costs one map per read and no memory per cell.
6. **A whole-column read walks only that column.** The evaluator gets a by-column index of its stored cells only if the existing map cannot be walked by column; its cost is one entry per stored cell, per evaluation, freed with the request.
7. **An encrypted xlsx (a CFB with `EncryptedPackage`) and an xls with `FILEPASS` throw** "This file is encrypted with a password. Encrypted files are not supported."
8. **A template fill drops XLM macro sheets, `xl/vbaData.xml`, `xl/activeX`, `customUI` ribbons and `oleObject*.bin` embeddings**, with their relationships and content types. It keeps package embeddings (an embedded xlsx or docx) and images.
9. **An XML part whose bytes start with a UTF-16 or UTF-32 byte order mark or with NULs, or whose declaration names an encoding other than UTF-8, throws naming the part.** A declared US-ASCII reads, because ASCII is UTF-8. The check is `xml::encoding` over the first 256 bytes `zip::Part` inflates of every `.xml`, `.rels` and `.vml` part: one 256-byte copy per part.
10. **A CSV in UTF-32 throws "UTF-32 is not supported"**, and a CSV that is not valid UTF-8 throws naming the charset it is not and the `encoding` option.
11. **A zip entry name or a relationship `Target` with `\` is read with `/`, and a `Target` is percent-decoded** before it is resolved.
12. **A workbook whose relationships omit sharedStrings is searched at `xl/sharedStrings.xml`.**
13. **An empty or whitespace `<v>` reads as `null`; a `t="d"` ISO value reads as its date or time type in the read's zone**; any other non-numeric number cell still throws naming the cell.
14. **An escaped surrogate pair (`_xD83D__xDE00_`) is joined into one character.**
15. **A `$` in a merge reference is stripped; phonetic `<rPh>` text is not part of a note.**
16. **An external link's `location` replaces any fragment its `Target` already has**, so the link has one `#`.
17. **A sheet whose part is missing is listed by `sheets()` with a `null` size** (`rows` and `columns` become `?uint`), and `rows()` on it throws naming the sheet.
18. **An xls STRING or LABEL record continued by CONTINUE is read across the CONTINUE**, as the SST path does; xls document properties decode by their property set's code page.
19. **An ods date with a UTC offset reads as the wall clock in the read's zone; `office:value-type="void"` reads as `null`; `office:string-value` is the cell's value.**
20. **ods cell text follows ODF 1.2 part 1 § 6.1.2**: whitespace collapses inside `text:p`, whitespace-only nodes outside it are ignored, and an empty `<text:p/>` is a line.
21. **A serial that names no real day (0 and 60 in the 1900 system) reads as its plain number**, as a negative serial does.
22. **Time arithmetic on a serial is checked; a result out of range reads as the plain number.**
23. **When no `separator` option is given, a first CSV line that is exactly `sep=` and one character sets the separator and is not a row.** It costs one line check per CSV read.
24. **The writer prefixes each future function by MS-XLSX § 2.2.2's list (`_xlfn.`, `_xlws.`), outside string literals**; the template filler leaves formulas as they are.
25. **`xml::attr` writes LF, CR and TAB as `&#10;`, `&#13;` and `&#9;`.**
26. **Text and sheet-name lengths count UTF-16 units, and sheet names compare by Unicode upper-case folding**, as Excel does.
27. **A table header cell is written as text of the same characters.**
28. **A validation accepts a whole-column (`A:A`) and a multi-range `sqref`; a quote in a list value is doubled**; a comma in a list value still throws, because Excel's list literal cannot contain one.
29. **The same image bytes are stored once per workbook**, found by a hash of the bytes; it costs one hash per image and one map entry per distinct image, per workbook.
30. **`createForFile` writes to a temporary name beside the target and renames it on `finish()`; on an error the temporary file is deleted.** It costs one rename.
31. **`Core\Csv::format` quotes a field that starts or ends with a space**; every reader reads the quoted field back the same, so this changes no data.
32. **The Spreadsheet CSV export writes a UTF-8 byte order mark and CRLF record ends (RFC 4180)**, because its reader is a spreadsheet program. `Core\Csv`'s own defaults stay LF with no BOM.
33. **CRLF comes from a new `Core\Csv::format` option, `lineEnd` (`"\n"` by default, or `"\r\n"`).** Rewriting LF after formatting would change line breaks inside quoted fields, and formatting one record per call costs a call per row; the option costs nothing where it is unused.
34. **A template fill moves notes (`ref`, VML `x:Row` and `x:Anchor`), page breaks (`<brk id>`), hyperlink `location`, threaded comments and pivot table and cache references with their rows.**
35. **A shared-formula group that crosses the repeated block is un-shared**: each follower is written with its own formula text.
36. **A filled template always sets `fullCalcOnLoad="1"`**, even where the template set `"0"`.
37. **The template shifter never shifts an external reference (`[1]Shop!A5`) and accepts column letters in either case.**
38. **A structured-reference item tests `#` before it reads `'` escapes**, so a column named `#Orders` is found.
39. **Date text in arithmetic counts from the workbook's date system**, 1900 or 1904.
40. **Text that only a locale makes a number (`"$1"`, `"1,000"`, `"1/15/2024"`) lists the cell as a coercion miss, and the cell keeps its stored value**; never a silent wrong value.
41. **Tests.** Each class is a Rust test or a conformance case that asserts values, parts or the thrown message. Cost tests compare the cost per unit at N and 8N measured in the same run, within 4×, as `crates/nvs-ext/tests/spreadsheet_read.rs:886` does (`rule:testing/a-test-holds-at-any-speed`, pattern 4). Test text uses neutral content (`Shop`, `example.com`).
42. **A pin that already has a test is not written twice.** Before writing one, the session greps the test files for its shape; where a test pins it, the item names that test and the check's name is replaced by it.
43. **Lanes.** Every stage appends to `crates/nvs-ext/tests/spreadsheet_read.rs`, `crates/nvs-ext/tests/spreadsheet_hostile.rs` or the extension's test files, and stages 1 to 5 share `extensions/spreadsheet/src/read/mod.rs` and the template code, so the stages run in order. No stage runs as a side goal.

## Stages

### 1. hostile input reads in linear time and memory, an encrypted file throws, a filled template keeps no macro part, and a part in another encoding throws

**Does:** closes the hostile-input and cost classes (decisions 1 to 10), with tests appended to
`crates/nvs-ext/tests/spreadsheet_hostile.rs`, driven through the component as that file's other cases
are, one conformance case and one attack.

- [x] SEC-04, RO-01: an ods valued cell with `number-columns-repeated` near 16,384 inside
  `number-rows-repeated` near 1,048,576 clones every repeated row. Yield the row again and never store
  the copies (decision 2). `extensions/spreadsheet/src/read/ods.rs:656`
  - [x] a whole-format row is a `sheet::Kept` with its repeat count, and `Cursor::Whole` gives each copy
  - [x] the rule and the ods module doc say a repeated row costs one row
  - [x] `an_ods_row_repeated_to_the_last_row_costs_one_row_to_read` passes
- [x] RX-21, ST-01: a single huge token is rescanned from its start on every 64 KiB refill. Resume the
  scan where the last refill stopped (decision 3). `extensions/spreadsheet/src/read/xml.rs:44`,
  `extensions/spreadsheet/src/read/sheet.rs:339`, `extensions/spreadsheet/src/read/mod.rs:283`
  - [x] `xml::next_from` with a `Resume`, used by the sheet cursor, `sheet::scan` and the shared strings
  - [x] a row the block ends inside is a `sheet::Partial` the cursor parses on, never again from its start
  - [x] `one_token_longer_than_many_read_blocks_is_scanned_once` passes
- [x] ST-03: the shared-string arena is bounded only by the ratio and the memory cap. Charge it to the
  archive-wide ceiling (decision 4). `extensions/spreadsheet/src/read/mod.rs:231`
  - [x] `Strings::within` stops the arena past `Directory::left`, and `Reader::strings` charges it
  - [x] `the_shared_string_table_counts_against_the_archive_ceiling` passes
- [x] `share` finds a shared-formula group by a linear search per cell; index groups by `si`.
  `extensions/spreadsheet/src/read/sheet.rs:210`
  - [x] a row's groups name each cell by its index, so `share` searches nothing (xlsx; xlsb's
    `binary.rs:812` search is over the groups still open, not the row, and is left)
  - [x] `many_shared_formula_groups_in_a_row_read_in_linear_time` passes
- [x] A custom format code is found by a linear search per xf; use a map.
  `extensions/spreadsheet/src/read/mod.rs:553`
  - [x] `styles` keeps the custom codes in a `HashMap` by id, the first code of an id kept
  - [x] `many_custom_number_formats_read_in_linear_time` passes
- [x] `rest()` allocates for each `g` and `a` in a format code, quadratic over a long code; slice
  without a copy. `extensions/spreadsheet/src/format/mod.rs:244`
  - [x] `lex` compares the next characters in place (`starts`), with no copy
  - [x] `a_long_format_code_with_many_g_and_a_letters_renders_in_linear_time` passes
- [x] FE-07: a whole-column read scans every stored cell in the row span; walk only that column
  (decision 6). `extensions/spreadsheet/src/eval/mod.rs:109`, `extensions/spreadsheet/src/eval/mod.rs:601`
  - [x] the evaluator's cells and formulas are each a `Grid` with a by-column row list, and a
    rect taller than it is wide walks its columns (`Stored`); `precedents` walks it too
  - [x] `a_whole_column_reference_visits_only_that_column` passes
- [x] SEC-13, RL-06: an encrypted xlsx says "it has no workbook stream", and an xls with FILEPASS is
  not detected and decodes cipher text. Both throw the message in decision 7.
  `extensions/spreadsheet/src/read/xls.rs:330`, `extensions/spreadsheet/src/read/xls.rs:344`
  - [x] `xls::read` throws `encrypted()` for a `cfb::File::has("EncryptedPackage")` and at `FILEPASS`
  - [x] `an_encrypted_xlsx_and_an_xls_with_a_password_throw_that_encryption_is_not_supported` passes
- [x] SEC-15, TP-09: a template fill keeps XLM macro sheets, `xl/vbaData.xml`, `xl/activeX`,
  `customUI` and `oleObject*.bin` against `rule:core-classes/spreadsheet-output-is-inert`. Drop them
  with their relationships and content types (decision 8). `extensions/spreadsheet/src/template/mod.rs:216`,
  `extensions/spreadsheet/src/template/mod.rs:1046`
  - [x] `dropped` names the parts; the content types, root, workbook and sheet relationships lose them
  - [x] `without_sheets` removes a macro sheet's `<sheet>` and its names, and moves `localSheetId`,
    `activeTab` and `firstSheet`
  - [x] `without_inert` removes a `control` or `oleObject` naming a dropped part, and its empty wrappers
  - [x] `a_filled_template_drops_macro_sheets_activex_ribbons_and_ole_objects_and_keeps_images` passes
- [x] A part declared or encoded as UTF-16 or UTF-32 is lexed as garbage with no error; throw naming
  the part (decision 9). `extensions/spreadsheet/src/read/xml.rs:1`
  - [x] `zip::Part` keeps the first 256 bytes of an XML part, and `xml::encoding` judges them
  - [x] `a_part_in_utf16_or_another_declared_encoding_throws_naming_the_part` passes
- [x] RC-03: a UTF-32LE byte order mark (`FF FE 00 00`) is read as UTF-16LE, and invalid UTF-8 in the
  first 4 KiB says "not a spreadsheet". Throw the messages in decision 10.
  `extensions/spreadsheet/nvs/Reader.nvs:486`
  - [x] `open` throws for a UTF-32 mark when no `encoding` is given, and `isUtf8` judges the first 4 KiB
  - [x] `csvText` throws `notUtf8()` for bytes that are not UTF-8 past the first 4 KiB
- [ ] The rules and pages a user reads say it: an encrypted file, a part in another encoding and a
  UTF-32 CSV throw (`docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:1`,
  `docs/examples/novis/Spreadsheet-Reader/open/about.md:1`, the doc comment at
  `extensions/spreadsheet/nvs/Reader.nvs:40`); a fill drops macro parts
  (`docs/rules/core-classes/spreadsheet-template-fill.md:1`, whose first sentence says every part is
  copied unchanged, `docs/rules/core-classes/spreadsheet-output-is-inert.md:1`,
  `docs/examples/novis/Spreadsheet-Template/fill/about.md:1`); and decision 2's memory cost is in
  the reads-as-a-stream rule. Then `bun nv rules --render` and `bun nv rules --check`.
- [x] Write the ten tests the `hostile-input` check names. The cost tests follow decision 41.
  `crates/nvs-ext/tests/spreadsheet_hostile.rs:1022`
- [x] Write the conformance case the `csv-charset` check names.
  `tests/conformance/novis/spreadsheet-a-utf32-csv-throws-and-a-csv-that-is-not-utf8-names-the-encoding-option.nvst:1`
- [ ] Attack `tests/hostile/novis/Spreadsheet-Reader/open/02-encrypted-and-wrongly-encoded-files.nvs`:
  an encrypted xlsx, an xls with a password, a sheet part in UTF-16, a UTF-32 CSV and an ods row
  repeated to the end of the grid. Build the bytes as `01-files-that-are-not-workbooks.nvs` in the same
  directory does, and follow `tests/hostile/README.md`. Run `bun nv proofs --run --id
  'Novis\Spreadsheet\Reader::open' --show` once: it passes.
- [x] Run the `hostile-input` and `csv-charset` checks' commands once: green.

### 2. xlsx reading resolves odd part names, reads empty and date cells, joins surrogate pairs, and lists a sheet whose part is missing

**Does:** fixes the xlsx reading findings (decisions 11 to 17, 21 and 22), with tests appended to
`crates/nvs-ext/tests/spreadsheet_read.rs`, whose helpers build each package (`:96` to `:455`).

- [ ] RX-02: backslash separators in zip entry names or relationship Targets, and `%XX` Targets, do not
  resolve (decision 11). `extensions/spreadsheet/src/read/zip.rs:180`, `extensions/spreadsheet/src/read/mod.rs:471`
- [ ] RX-03: a package whose workbook relationships omit sharedStrings is not searched for
  `xl/sharedStrings.xml` (decision 12). `extensions/spreadsheet/src/read/mod.rs:717`
- [ ] RX-07, RX-11: `<c><v></v></c>` throws and stops the read, and `t="d"` reads as text
  (decision 13). `extensions/spreadsheet/src/read/mod.rs:1222`
- [ ] RX-10: `_xD83D__xDE00_` stays literal (decision 14). `extensions/spreadsheet/src/read/xml.rs:219`
- [ ] RX-16: a `$` in a merge reference makes `merged()` throw; RX-17: `<rPh>` text in a comment is
  appended to the note (decision 15). `extensions/spreadsheet/src/read/merge.rs:57`,
  `extensions/spreadsheet/src/read/extras.rs:325`
- [ ] RX-18: an external Target with `#` plus a `location` gives two `#` (decision 16).
  `extensions/spreadsheet/src/read/extras.rs:229`
- [ ] ST-06: `sheets()` throws for the whole workbook when one sheet's part is missing (decision 17).
  `rows` and `columns` become optional in `wit/spreadsheet.wit:496` and in `SheetInfo` at
  `extensions/spreadsheet/nvs/Reader.nvs:14`. `extensions/spreadsheet/src/read/mod.rs:866`
- [ ] DT-01: serial 60 reads as 1900-03-01 and serial 0 as 1899-12-31 (decision 21).
  `extensions/spreadsheet/src/read/mod.rs:1309`
- [ ] DT-06: `ms * 1_000_000` overflows `i64` above serial 106,751 under a `[h]` format and wraps
  silently in release (decision 22). `extensions/spreadsheet/src/read/mod.rs:1286`
- [ ] The read table and the sheet list say it: `null` for an empty value, the type of a `t="d"`
  cell, serials 0 and 60 and an out-of-range duration as plain numbers
  (`docs/rules/core-classes/spreadsheet-value-types.md:1`), and a `null` size for a missing sheet part
  (`docs/rules/core-classes/spreadsheet-roster.md:1`, `docs/examples/novis/Spreadsheet-Reader/sheets/about.md:1`,
  the doc comment at `extensions/spreadsheet/nvs/Reader.nvs:73`). Then `bun nv rules --render` and
  `bun nv rules --check`.
- [ ] Write the eleven tests the `xlsx-reading` check names. `crates/nvs-ext/tests/spreadsheet_read.rs:4251`
- [ ] Run the `xlsx-reading` check's command once: green.

### 3. xls reads continued text and code pages, ods reads offsets, void cells and whitespace as ODF says, and a CSV sep= line sets the separator

**Does:** fixes the xls, ods and CSV reading findings (decisions 18 to 20 and 23), with tests appended
to `crates/nvs-ext/tests/spreadsheet_read.rs` (its `xls_of` at `:2320` and `ods` at `:2326` build the
files) and one conformance case.

- [ ] RL-01: a STRING or LABEL record longer than 8,224 bytes and continued by CONTINUE throws
  (decision 18). `extensions/spreadsheet/src/read/xls.rs:724`, `extensions/spreadsheet/src/read/xls.rs:113`
- [ ] RL-02: xls property strings decode as Windows-1252 whatever the property set's `CODEPAGE` says
  (decision 18). `extensions/spreadsheet/src/read/xls_extras.rs:389`
- [ ] RO-02: an ods date with a UTC offset parses its seconds as 0; `void` reads as empty text; a
  `string-value` with no `text:p` is ignored (decision 19). `extensions/spreadsheet/src/read/ods.rs:86`
- [ ] RO-03: whitespace between the elements of a pretty-printed `content.xml` lands in cell text, and
  an empty `<text:p/>` is not counted (decision 20). `extensions/spreadsheet/src/read/ods.rs:564`,
  `extensions/spreadsheet/src/read/ods.rs:535`
- [ ] RC-04: a first line `sep=;` is returned as a data row (decision 23).
  `extensions/spreadsheet/nvs/Reader.nvs:44`
- [ ] The pages a user reads say it: `void` reads as `null` and an offset date as the zone's wall clock
  (`docs/rules/core-classes/spreadsheet-value-types.md:1`); a `sep=` line sets the separator
  (`docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:1`,
  `docs/examples/novis/Spreadsheet-Reader/open/about.md:1`, the doc comment at
  `extensions/spreadsheet/nvs/Reader.nvs:40`). Then `bun nv rules --render` and `bun nv rules --check`.
- [ ] Write the six tests the `xls-ods-reading` check names. `crates/nvs-ext/tests/spreadsheet_read.rs:4251`
- [ ] Write the conformance case the `csv-separator-line` check names.
  `tests/conformance/novis/spreadsheet-a-csv-that-starts-with-sep-uses-that-separator.nvst:1`
- [ ] Run the `xls-ods-reading` and `csv-separator-line` checks' commands once: green.

### 4. the writer prefixes new functions, keeps line breaks in attributes, counts lengths as Excel does, stores an image once, and a CSV export opens cleanly in Excel

**Does:** fixes the writing findings (decisions 24 to 33), with tests appended to
`extensions/spreadsheet/tests/write.rs` and four conformance cases.

- [ ] WX-11: no `_xlfn.` or `_xlws.` prefix is ever written, so XLOOKUP, IFS, CONCAT, TEXTJOIN and
  every function newer than Excel 2007 open as `#NAME?` (decision 24). Keep the list as a sorted
  table in the writer, from MS-XLSX § 2.2.2. `extensions/spreadsheet/src/cells.rs:384`
- [ ] WX-01: attribute values keep LF and TAB raw, so validation prompts, link tooltips, table column
  names and style names lose their line breaks (decision 25). `extensions/spreadsheet/src/xml.rs:24`
- [ ] WX-06, WX-07: text length (32,767) and sheet-name length (31) count chars, and the duplicate-name
  check folds with `to_lowercase` (decision 26). `extensions/spreadsheet/src/cells.rs:162`,
  `extensions/spreadsheet/src/writer.rs:176`
- [ ] WX-15: a numeric table header cell is written as a number while its `tableColumn` name is text
  (decision 27). `extensions/spreadsheet/src/table.rs:164`
- [ ] WX-16: a whole-column or multi-range `sqref` is rejected, and a list value with a quote throws
  (decision 28). `extensions/spreadsheet/src/tail.rs:111`
- [ ] WX-20: the same image bytes added twice are stored twice (decision 29).
  `extensions/spreadsheet/src/writer.rs:631`
- [ ] WX-27: `createForFile` leaves a half-written file when a write fails (decision 30). Check whether
  `Export::save` writes through it, and give it the same behaviour if not.
  `extensions/spreadsheet/nvs/Workbook.nvs:77`
- [ ] WC-01: a CSV field with a leading or trailing space is not quoted (decision 31). Rename and
  extend `tests/conformance/core/csv-format-quotes-exactly-the-separator-the-quote-cr-and-lf.nvst` to
  the name the `csv-and-file-export` check gives. `crates/nvs-stdlib/src/csv.rs:983`
- [ ] WC-03: the Spreadsheet CSV export has no UTF-8 byte order mark and ends records with LF
  (decisions 32 and 33). Add `lineEnd` to `Core\Csv::format`'s options and its help text, and pass
  `"\r\n"` and the BOM from the export. `crates/nvs-stdlib/src/csv.rs:397`,
  `extensions/spreadsheet/nvs/Workbook.nvs:656`
- [ ] The rules and pages a user reads say it: lengths in UTF-16 units and the upper-case name fold
  (`docs/rules/core-classes/spreadsheet-bulk-boundary.md:8`); a future function is written with its
  prefix (`docs/rules/core-classes/spreadsheet-formula-is-a-value.md:1`); a numeric table header, the
  validation ranges and one stored copy per image, with its cost
  (`docs/rules/core-classes/spreadsheet-sheet-end-parts.md:1`,
  `docs/examples/novis/Spreadsheet-Sheet/validate/about.md:1`,
  `docs/examples/novis/Spreadsheet-Sheet/image/about.md:1`); the temporary file and its rename
  (`docs/rules/core-classes/spreadsheet-writes-as-a-stream.md:1`,
  `docs/examples/novis/Spreadsheet-Workbook/createForFile/about.md:1`); the CSV export's BOM and
  CRLF (find its rule with `bun nv brief --where csv`; `spreadsheet-evaluation-and-roster` says CSV
  is written through `Core\Csv`); and `Core\Csv::format`'s edge-space quoting and `lineEnd`
  (`crates/nvs-stdlib/src/csv.rs:1`, `docs/examples/core/Csv/format/about.md:1`). Then
  `bun nv rules --render` and `bun nv rules --check`.
- [ ] Write the eight tests the `writing` check names. `extensions/spreadsheet/tests/write.rs:964`
- [ ] Write the four conformance cases the `csv-and-file-export` check names, the edge-space one by
  the rename above. `tests/conformance/novis/spreadsheet-a-failed-write-to-a-file-leaves-no-file.nvst:1`,
  `tests/conformance/novis/spreadsheet-a-csv-export-starts-with-a-bom-and-ends-records-with-crlf.nvst:1`,
  `tests/conformance/core/csv-format-ends-each-record-with-the-line-end-option.nvst:1`
- [ ] Run the `writing` and `csv-and-file-export` checks' commands once: green.

### 5. a template fill moves every part that points at a row, and the evaluator reads escaped table columns, 1904 date text and locale text

**Does:** fixes the template-fill and evaluator findings (decisions 34 to 40), with tests appended to
`extensions/spreadsheet/tests/template.rs` and `extensions/spreadsheet/tests/evaluate.rs`.

- [ ] TP-02: a fill does not move notes (`comments*.xml` `ref`, VML `x:Row` and `x:Anchor`), page
  breaks, hyperlink `location`, threaded comments or pivot table and cache references (decision 34).
  `extensions/spreadsheet/src/template/mod.rs:374`, `extensions/spreadsheet/src/template/mod.rs:1041`
- [ ] TP-05: a shared-formula group spanning the repeated block gives wrong references for followers
  below it (decision 35). `extensions/spreadsheet/src/template/mod.rs:1470`
- [ ] TP-08: an explicit `fullCalcOnLoad="0"` is kept (decision 36).
  `extensions/spreadsheet/src/template/mod.rs:500`
- [ ] FP-08: the shifter reads `[1]Shop!A5` as local `Shop!A5` and shifts it; and it accepts only
  upper-case column letters, so `a5` is not shifted (decision 37).
  `extensions/spreadsheet/src/template/shift.rs:138`, `extensions/spreadsheet/src/template/shift.rs:50`
- [ ] FP-06: a structured-reference column named `#Orders` (stored `'#Orders`) fails as an item
  (decision 38). `extensions/spreadsheet/src/eval/parse.rs:848`
- [ ] `iso_day` counts from 1900, so `="2024-01-15"+0` in a 1904 workbook is wrong (decision 39).
  `extensions/spreadsheet/src/eval/value.rs:200`
- [ ] FE-03, FE-04: `"$1"`, `"1,000"` and `"1/15/2024"` give `#VALUE!` in arithmetic and match nothing
  as a criterion, with no miss listed (decision 40). The miss needs a kind of its own: add it to
  `Kind` and to `missed-info` at `wit/spreadsheet.wit:602` and on the Novis side.
  `extensions/spreadsheet/src/eval/value.rs:157`, `extensions/spreadsheet/src/eval/mod.rs:46`
- [ ] The rules and pages a user reads say it: what a fill moves and that it always recalculates
  (`docs/rules/core-classes/spreadsheet-template-fill.md:1`,
  `docs/examples/novis/Spreadsheet-Template/fill/about.md:1`), and the new miss kind
  (`docs/rules/core-classes/spreadsheet-evaluation-and-roster.md:1`,
  `docs/examples/novis/Spreadsheet-Reader/evaluate/about.md:1`). Then `bun nv rules --render` and
  `bun nv rules --check`.
- [ ] Write the six tests the `formulas-and-fill` check names for template fill.
  `extensions/spreadsheet/tests/template.rs:870`
- [ ] Write the three tests it names for the evaluator. `extensions/spreadsheet/tests/evaluate.rs:685`
- [ ] Run the `formulas-and-fill` check's command once: green.

### 6. the hostile and reading classes the engine already handles are pinned by tests

**Does:** writes one test per class the triage found handled, of the shape the libraries got wrong. A
red test is a bug: fix its cause in this stage, or, if the fix does not fit, add it to § *Backlog*
with its test's name. Decision 42 applies to every item.

**Split:** two writing subagents side by side, one per Rust file, each given its list below and the
file's helpers. Neither builds. The session writes the four CSV cases and the attack, then builds and
runs the checks once both end.

- [ ] `crates/nvs-ext/tests/spreadsheet_hostile.rs:1022`, the seven tests the `pinned-hostile` check
  names: SEC-01 an external entity in an ods part and a template part, and in UTF-16, double-encoded
  (`&#38;#60;!DOCTYPE`) and declaration-after-comment forms; SEC-02 nested entities in an ods part;
  SEC-03 a bomb in a relationships part and in an ods `content.xml`; SEC-06 a DIFAT cycle, a mini-FAT
  cycle and a short stream; SEC-07 Targets `../../x`, `/x`, `C:\x`, `file:///x` and `http://x`, none
  read; SEC-09 a relationship cycle and a self reference; SEC-14 a chain of 100,000 cells each naming
  the next, evaluated.
- [ ] `crates/nvs-ext/tests/spreadsheet_read.rs:4251`, the fifteen tests the `pinned-reading` check
  names: RX-01 `x:`-prefixed and Strict OOXML namespaces; RX-04 cells and rows without `r`; RX-05 a
  missing and a wrong dimension; RX-08 `<si/>`, `rPh` and a missing count; RX-09 several `<t>` in
  `<is>`; RX-12 a shared formula whose master is outside the rows read; RX-13 `0`, `1`, `true` and
  `false` flags; RX-14 sheet order from `workbook.xml`; RX-22 bytes before the zip give the clear
  error; RB-01 truncated xlsb records (`an_xlsb_record_whose_length_lies_throws` in the hostile file
  may already pin it); RL-03 an array constant, a quote in a string and a deleted-sheet reference in
  an xls formula; RL-04 the RK value 0.07, which also pins NM-02; RO-04 an ods without `mimetype`;
  RO-05 a sheet name with `.`; RO-07 a number style with three leading-zero digits.
- [ ] The four conformance cases the `pinned-csv` check names: SEC-10 a CSV export of `=1+1`, `+1`,
  `-1`, `@x`, a leading TAB and a leading CR gets the apostrophe `Workbook.nvs:724` adds; RC-01 an
  unterminated quote at the end of the file, RC-02 a backslash read as a literal by default and RC-05
  a bare CR; RC-07 `007`, a 16-digit id, `1E3`, `+49`, `1.10` and `TRUE` stay strings; RC-08 a
  32,767-character field. `tests/conformance/novis/spreadsheet-a-csv-export-prefixes-an-apostrophe-to-text-that-starts-a-formula.nvst:1`,
  `tests/conformance/novis/spreadsheet-reads-csv-quotes-backslashes-and-a-bare-cr-as-written.nvst:1`,
  `tests/conformance/novis/spreadsheet-csv-values-that-look-like-numbers-stay-strings.nvst:1`,
  `tests/conformance/novis/spreadsheet-a-csv-field-of-32767-characters-reads-whole.nvst:1`
- [ ] ST-07: output after the `finish()` of a download. The triage says the program does not
  compile; check it once with `bun nv try`. Write the attack
  `tests/hostile/novis/Spreadsheet-Export/download/02-output-after-the-download.nvs`, with
  `// hostile: expect-refusal` if it does not compile, and run `bun nv proofs --run --id
  'Novis\Spreadsheet\Export::download' --show` once: it passes.
- [ ] Run the `pinned-hostile`, `pinned-reading` and `pinned-csv` checks' commands once: green.

### 7. the date, number, format, formula and writing classes are pinned by tests, and every declined class is in its rule's Declined list

**Does:** the same as stage 6 for the rest of the triage, and records each declined class in the
Declined list of the rule that owns it.

**Split:** five writing subagents side by side, one per file group below: `spreadsheet_read.rs`,
`format.rs`, `evaluate.rs`, `template.rs`, and `write.rs` with `sheet_end.rs`. None builds. The session
writes the Declined entries, then builds and runs both checks once all end.

- [ ] `crates/nvs-ext/tests/spreadsheet_read.rs:4251`, the five tests the `pinned-dates` check names:
  DT-02 cell values in a 1904 xlsx; DT-03 a wall clock in a daylight-saving gap and one in an overlap;
  DT-04 milliseconds read into a time of day; NM-03 an xlsx percent cell; FE-16 a cached `t="str"`
  result `00123` stays text.
- [ ] `extensions/spreadsheet/tests/format.rs:265`, four tests: NF-01 `[RED]`, `GENERAL` and
  `[<=9999999]###-####;(###) ###-####`; NF-03 `?/0`; NF-06 `[$-F800]`; NF-07 a 50,000-character code
  in linear time (decision 41).
- [ ] `extensions/spreadsheet/tests/evaluate.rs:685`, twelve tests: FP-01 apostrophe, `!` and
  non-Latin sheet names; FP-02 `Sheet!$B:$B` and `A1:B2:C3`; FP-04 `SUM(1,)` and `IF` with an empty
  argument; FP-05 a string that looks like a reference; FP-07 names `A1B`, `R1C`, `x.y` and non-ASCII;
  FP-09 `@` and `A1#` listed; FP-10 `1<2<3`; FE-01 `""=0` is false; FE-02 `#REF!` inside a range; FE-06
  IFS, SWITCH and CHOOSE evaluate only the branch taken; FE-09 `ROUNDDOWN(249.9975,0)` is 249 and
  `ROUNDUP(280.000000000001,0)` is 281; FE-12 SUBTOTAL over hidden rows and a nested SUBTOTAL.
- [ ] `extensions/spreadsheet/tests/template.rs:870`, three tests: TP-03 relative and quoted-sheet
  names; TP-04 a quoted sheet name that looks like a cell; TP-10 media, a header or footer image and
  note VML copied unchanged.
- [ ] `extensions/spreadsheet/tests/write.rs:964` and `extensions/spreadsheet/tests/sheet_end.rs:1307`,
  six tests: WX-08 one row with gaps written in column order and WX-10 a column-only and a both-axis
  freeze in `write.rs`; WX-18 an internal link to a quoted sheet, WX-19 a note of several lines, WX-28
  a merge over the autofilter row and WX-24 protection XML with no `lockRevision` in `sheet_end.rs`.
- [ ] Declined, in `docs/rules/core-classes/spreadsheet-writes-as-a-stream.md:96`: WX-09
  `[Content_Types].xml` first in the zip (the parts are known only at finish, OPC does not require
  the order, and writing it first would hold the workbook); WX-25 `<dimension>` (the sheet head is
  written before the row count is known, and the element is optional); WX-12 shared strings (inline
  strings keep the writer O(1) per cell, and Excel, LibreOffice and Sheets read them).
- [ ] Declined, in `docs/rules/core-classes/spreadsheet-sheet-end-parts.md:103`: WX-21 chart number
  caches (a chart may name rows already streamed out, caching would hold every charted value, and
  Excel and LibreOffice recompute from the cells); WX-17 `stopIfTrue` and `containsText` (the API does
  not offer them, so nothing is written wrong); SEC-16 non-ASCII protection passwords (the legacy hash
  is documented as no security, and the salted hash would add entropy against
  `rule:core-classes/spreadsheet-output-is-inert`).
- [ ] Declined, in `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:166`: RX-04 rows with a
  decreasing or duplicate `r` keep throwing "out of order" (a clear error, and the reader stays one
  forward pass); RX-22 bytes before the zip keep the clear "not a spreadsheet" error.
- [ ] Declined, in `docs/rules/core-classes/spreadsheet-template-fill.md:71`: TP-01 3D references
  (`Jan:Mar!C5`) stay unshifted (a fill repeats rows on one sheet, and Excel's own rule for a 3D span
  is per sheet).
- [ ] Declined, in `docs/rules/core-classes/spreadsheet-evaluation-and-roster.md:95`: FE-13, FE-14
  arrays, dynamic arrays, RATE, XIRR and the distributions are listed, never computed.
- [ ] Declined, in `docs/rules/core-classes/spreadsheet-styles-and-defaults.md:68`: WX-26 auto width
  from the first 1,000 rows is the stream's design.
- [ ] Confirm each owner with `bun nv brief --where <keyword>` before writing, then
  `bun nv rules --render` and `bun nv rules --check`.
- [ ] Run the `pinned-dates` and `pinned-values` checks' commands once: green.

### 8. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 has seven of its fourteen items done and committed. 9597f6440 and f0c22cade landed the
repeated ods row, the one-pass XML scan, the shared-string ceiling and the shared-formula index.
4e38a9cc1 landed the custom-format map (`read/mod.rs`'s `styles`), the in-place keyword test in
`format::lex` (`starts`), and the evaluator's `Grid<V>` (`eval/mod.rs`). Each sheet's cells and
formulas now carry a by-column row list, and a rect taller than it is wide walks only its
columns (`Stored`), in `Ctx::entries`, `Ctx::cells` and `precedents` alike. b47fe3941 makes
`xls::read` throw decision 7's message for a compound file with `EncryptedPackage`
(`cfb::File::has`) and at an xls `FILEPASS` record. Seven of the ten `hostile-input` tests exist
and pass, each run once by name. `spreadsheet_hostile.rs` gained `xlsx_with` and `styles`, a styles
part of custom codes; `linear_by`, timing with any function; `evaluate_time`, which runs
`readerEvaluate`; and `compound_of`, a compound file with a named stream and its content.

The sweep's reds: `env-secret-proofs` is stale (defined nowhere, no log), as two handoffs found
already. The nightly reds on run 41813182715 (f3150b922) are fixed by a8bdf37fb (macOS `live_edit`)
and a2f451fad (the Miri `openat` skip); the next nightly confirms both.

The tree still holds another session's uncommitted edits in `crates/nvs-lsp/`,
`crates/nvs-diagnostics/` and now `crates/nvs-hir/`. They fail workspace `cargo clippy
--all-targets` on a `u128 as u64` cast at `crates/nvs-diagnostics/src/source.rs:374`. They were
left unstaged, and the spreadsheet crate's own clippy is clean.

### Next

The template item (decision 8), `extensions/spreadsheet/src/template/mod.rs`. Dropping the parts
is not enough, because three other parts name them:
- `dropped` (`:216`) adds `xl/macrosheets/`, `xl/vbadata.xml`, `xl/activex/`, `customui/` and
  `oleobject` (`xl/embeddings/oleObject*.bin`), with each `_rels` of a dropped part.
- `rewritten` (`:1050`) drops their relationships and content types through `without` (`:471`):
  `[Content_Types].xml` (`macrosheet`, `vbadata`, `activex`, `oleobject`, `customui`),
  `xl/_rels/workbook.xml.rels` (`macrosheet`), and the root `_rels/.rels`, which names
  `customUI` (`ui/extensibility`). A sheet's own `.rels` names its `control` and `oleObject`
  relationships, and today `.rels` parts return `Ok(None)` (copied unchanged), so add an arm for
  `xl/worksheets/_rels/`.
- `xl/workbook.xml` keeps a `<sheet r:id>` for each macro sheet. `Template::open` (`:557`) skips
  them already. The fill removes that `<sheet>` element, removes each `definedName` whose
  `localSheetId` is that sheet, lowers each higher `localSheetId` by one, and clamps
  `workbookView`'s `activeTab` and `firstSheet`.
- A worksheet's XML names its ActiveX controls in `<controls>` and its OLE objects in
  `<oleObjects>`, each inside `mc:AlternateContent`. Remove both elements where the sheet is
  written (`head`, `:1103`, and the path that copies a sheet without blocks), or the file does
  not open.

Its test is `a_filled_template_drops_macro_sheets_activex_ribbons_and_ole_objects_and_keeps_images`
in `crates/nvs-ext/tests/spreadsheet_hostile.rs`. Build the template with `zip` (`:44`), fill it
as `filling` (`:968`) does, and read the archive's parts with `entry` (`:1066`). Then the
encodings item (decision 9, `extensions/spreadsheet/src/read/xml.rs:1`) and RC-03 (decision 10,
`extensions/spreadsheet/nvs/Reader.nvs:486`). No existing test is expected to go red.

### Backlog
