---
looper: 1
title: "the PDF engine handles every edge case dompdf, mPDF, TCPDF and WeasyPrint got wrong"
summary: "every failure class from the four engines' trackers and advisories is pinned by a green test or attack, and each one the engine does not support is reported in the render result"
context: [extensions/pdf/subset.json, docs/rules/core-classes/pdf-css-subset.md, docs/rules/core-classes/pdf-tables-across-pages.md, extensions/pdf/tests/paginate.rs]
stages:
  - id: 1
    title: "hostile CSS, images, SVG and link text never break a render, and each bomb ends at the request's ceiling"
    status: done
    context: [extensions/pdf/src/css/value.rs, extensions/pdf/src/write.rs, extensions/pdf/src/assets.rs, extensions/pdf/src/layout/image.rs, tests/hostile/README.md, tests/hostile/novis/Pdf-Document/asset/01-files-and-addresses-outside-the-assets.nvs]
  - id: 2
    title: "no part of a render grows faster than the document: tables, margin-box lookups, repeated images and selector matching are linear"
    status: done
    context: [extensions/pdf/src/layout/table.rs, extensions/pdf/src/layout/margin.rs, extensions/pdf/src/layout/image.rs, extensions/pdf/src/css/mod.rs, extensions/pdf/tests/layout.rs]
  - id: 3
    title: "an <img> and an inline <svg> are inline boxes in their line, with vertical-align, float, max and min sizes and EXIF orientation"
    status: done
    context: [extensions/pdf/src/layout/inline.rs, extensions/pdf/src/layout/image.rs, extensions/pdf/src/layout/text.rs, extensions/pdf/src/layout/block.rs]
  - id: 4
    title: "page breaks fall where CSS Fragmentation puts them: parity by direction, kept margins, no padding-only page, floats after a break, tall boxes that continue"
    status: done
    context: [extensions/pdf/src/layout/block.rs, extensions/pdf/src/layout/text.rs, extensions/pdf/src/layout/flex.rs, extensions/pdf/src/paginate/mod.rs]
  - id: 5
    title: "tables read col widths, row breaks and HTML span numbers, resolve collapsed borders by CSS 2.1, and run right to left"
    status: open
    context: [extensions/pdf/src/layout/table.rs, extensions/pdf/src/layout/inline.rs, extensions/pdf/src/layout/flex.rs, extensions/pdf/tests/tables.rs, docs/rules/core-classes/pdf-tables-across-pages.md]
  - id: 6
    title: "every text and box property the subset accepts is applied: overflow clips, visibility hides, margins go negative, span-level white-space and line-height, list types"
    status: open
    context: [extensions/pdf/src/layout/inline.rs, extensions/pdf/src/layout/text.rs, extensions/pdf/src/layout/block.rs, extensions/pdf/src/layout/boxes.rs, extensions/pdf/src/css/mod.rs]
  - id: 7
    title: "page counters restart, unknown page selectors and @media features are reported, and margin boxes report their overflow"
    status: open
    context: [extensions/pdf/src/layout/margin.rs, extensions/pdf/src/paginate/mod.rs, extensions/pdf/src/css/mod.rs, extensions/pdf/src/lib.rs]
  - id: 8
    title: "the pagination and table classes the engine already handles are pinned by tests"
    status: open
    context: [extensions/pdf/tests/paginate.rs, extensions/pdf/tests/tables.rs]
  - id: 9
    title: "the text, image, CSS and output classes the engine already handles are pinned by tests"
    status: open
    context: [extensions/pdf/tests/layout.rs, extensions/pdf/tests/css.rs, extensions/pdf/tests/render.rs, extensions/pdf/tests/read.rs]
  - id: 10
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: hostile-input
    stage: 1
    name: non-finite lengths, var() chains, size-0 text, odd link and metadata text, encoded schemes, SVG entities and use cycles all render or throw cleanly
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, hostile_input]
    timeout: 1800
    tests:
      - a_length_that_overflows_to_infinity_is_dropped_and_reported
      - a_page_size_that_overflows_to_infinity_falls_back_to_a4_and_is_reported
      - a_var_chain_that_doubles_at_each_step_resolves_each_property_once
      - text_with_a_font_size_of_zero_draws_nothing_and_writes_no_nan
      - a_link_and_a_title_with_parentheses_backslashes_and_non_ascii_survive_a_round_trip
      - a_percent_encoded_scheme_is_a_missing_asset
      - an_svg_with_entities_and_a_use_cycle_renders_or_throws
  - id: linear-cost
    stage: 2
    name: rows, pages with named strings, repeated images and rules each cost the same per unit at 10x the count
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, cost]
    timeout: 1800
    tests:
      - a_table_ten_times_longer_costs_ten_times_the_work
      - a_string_set_on_every_page_costs_the_same_per_page_at_ten_times_the_pages
      - one_image_used_a_thousand_times_is_resolved_and_decoded_once
      - rules_that_cannot_match_an_element_are_not_tested_against_it
  - id: inline-images
    stage: 3
    name: an image sits in its line, aligns and floats, honours max and min sizes, and turns by its EXIF orientation
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, images]
    timeout: 1800
    tests:
      - an_img_in_a_paragraph_sits_on_the_line_between_its_words
      - vertical_align_middle_centres_an_img_on_the_line
      - a_floated_img_inside_a_link_wraps_the_text_beside_it
      - display_block_makes_an_img_its_own_block
      - max_width_max_height_and_a_percent_height_limit_an_img
      - an_img_taller_than_the_page_moves_whole_to_the_next_page_and_shrinks_to_fit
      - a_jpeg_with_an_exif_orientation_is_drawn_turned
  - id: breaks
    stage: 4
    name: page parity follows direction, margins survive forced breaks, no page holds only a bottom edge, a float after a break moves with it, tall boxes and flex bands continue
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, breaks]
    timeout: 1800
    tests:
      - in_a_right_to_left_document_the_first_page_is_a_left_page
      - recto_and_verso_follow_the_document_direction
      - a_forced_break_keeps_the_top_margin_of_the_next_box
      - a_box_whose_content_fills_the_page_adds_no_page_for_its_bottom_padding
      - heights_that_sum_to_the_page_in_another_order_still_fit
      - a_float_after_a_forced_break_lands_on_the_new_page
      - a_heading_before_a_block_taller_than_a_page_moves_with_its_first_lines
      - a_fixed_height_box_taller_than_a_page_continues_on_the_next_page
      - a_flex_line_taller_than_a_page_splits_between_its_items_lines
  - id: tables
    stage: 5
    name: col widths, row and row-group breaks, HTML span parsing, CSS 2.1 border conflicts, header dropping and right-to-left tables
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, tables]
    timeout: 1800
    tests:
      - col_and_colgroup_widths_set_the_column_widths
      - break_before_page_on_a_row_or_row_group_starts_a_new_page
      - rowspan_zero_spans_to_the_end_of_the_row_group
      - a_colspan_with_trailing_text_reads_its_leading_digits
      - collapsed_borders_resolve_by_width_then_style_then_owner
      - a_header_and_footer_taller_than_the_page_are_left_off_that_page
      - a_right_to_left_table_runs_its_columns_from_the_right
      - a_right_to_left_flex_row_runs_from_the_right
  - id: boxes
    stage: 6
    name: overflow, visibility, negative margins, span-level white-space and line-height, text-align-last and list types are applied or reported
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, boxes]
    timeout: 1800
    tests:
      - overflow_hidden_clips_content_and_adds_no_page_for_it
      - visibility_hidden_keeps_the_space_and_draws_nothing
      - a_negative_margin_pulls_the_box_up
      - collapsing_margins_add_the_largest_positive_and_the_most_negative
      - white_space_nowrap_on_a_span_keeps_its_words_on_one_line
      - line_height_on_a_span_grows_its_line
      - text_align_last_aligns_the_last_line_or_is_reported
      - ol_type_and_reversed_set_the_markers
  - id: pages
    stage: 7
    name: counter-reset page restarts the numbers, unknown page selectors and media features are reported, a tall margin box is listed in overflows, corners align by the spec
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, pages]
    timeout: 1800
    tests:
      - counter_reset_page_restarts_the_page_number
      - an_unknown_page_pseudo_class_is_reported_as_dropped
      - a_media_query_is_applied_by_type_and_page_size_and_an_unknown_feature_is_reported
      - a_stylesheet_that_is_not_utf8_is_reported_as_dropped
      - a_margin_box_taller_than_its_margin_is_listed_in_overflows
      - corner_margin_boxes_align_toward_the_page
  - id: pinned-pagination
    stage: 8
    name: every pagination and table class the engine already handles has a test
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, edge_pagination, --test, edge_tables]
    timeout: 1800
    tests:
      - two_forced_breaks_in_a_row_make_one_break
      - a_trailing_break_after_adds_no_page
      - a_forced_break_at_the_top_of_a_page_adds_no_page
      - a_break_inside_a_fixed_element_is_ignored
      - a_break_after_passes_up_through_wrapper_elements
      - an_unforced_break_drops_the_adjoining_margin_inside_a_flow_root
      - a_chain_of_three_avoid_boxes_moves_together
      - orphans_and_widows_split_seven_lines_as_css_says
      - a_paragraph_split_across_pages_keeps_every_character_once
      - a_split_list_item_draws_its_marker_on_the_first_page_only
      - list_numbers_and_counters_continue_across_pages
      - fixed_content_repeats_on_pages_made_only_by_forced_breaks
      - a_blank_page_between_named_groups_has_the_outer_size
      - a_multi_row_thead_repeats_whole
      - degenerate_tables_render_without_a_panic
      - fixed_table_layout_and_percentages_over_a_hundred_share_the_width
      - a_rowspan_cell_aligns_its_content_by_vertical_align
      - a_nested_table_splits_across_pages_with_its_own_borders
      - a_float_inside_a_cell_stays_inside_the_cell
      - a_link_in_a_repeated_header_is_a_link_on_every_page
  - id: pinned-text-output
    stage: 9
    name: every text, image, CSS and output class the engine already handles has a test
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, edge_text, --test, edge_output]
    timeout: 1800
    tests:
      - a_family_listed_first_that_has_digits_draws_them
      - cjk_kinsoku_keeps_closing_punctuation_off_a_line_start
      - break_word_breaks_a_long_word_that_starts_after_an_inline_element
      - pre_drops_the_first_newline_and_keeps_leading_spaces
      - an_inline_block_ending_in_a_space_keeps_its_words_on_one_line
      - entities_are_decoded_in_text_and_attributes
      - important_in_a_shorthand_wins_for_every_longhand
      - border_box_and_percent_vertical_margins_follow_css
      - a_float_wider_than_its_container_sits_alone_on_its_line
      - list_markers_go_past_z_and_past_3999
      - one_given_image_dimension_keeps_the_aspect_ratio
      - cmyk_jpeg_sixteen_bit_and_interlaced_png_render
      - background_size_cover_and_contain_fill_the_box
      - a_link_wrapped_over_two_lines_has_a_rectangle_on_each
      - an_element_split_across_pages_is_one_structure_element
      - a_merged_rotated_landscape_page_keeps_its_size_and_rotation
      - a_password_and_a_title_with_parentheses_and_percent_round_trip
      - invoice_xml_with_xpacket_text_embeds_intact
---

# the PDF engine handles every edge case dompdf, mPDF, TCPDF and WeasyPrint got wrong

## Why

The user asked for a deep review of the three most used PHP HTML-to-PDF libraries (dompdf, mPDF,
TCPDF and its successor tc-lib-pdf), and WeasyPrint was added because it uses the same CSS Paged Media
model as `Novis\Pdf`. On 2026-11-04 five research subagents read the issue trackers (about 7,000
issues), the libraries' test suites and the security advisories, and grouped them into failure
classes. Three audit subagents then read `extensions/pdf/` against each class. This goal is the result.
It holds everything a session needs; no session re-reads the trackers.

The classes that every library hit, ranked by how often: a table across pages (header repeat, a row
taller than a page, a rowspan across a break, borders at the break), `break-inside: avoid` and
`break-after: avoid` that cannot be met, blank pages (a trailing break, content that ends at the page
bottom, an image taller than the page), page counters and running headers that need a second pass,
floats and positioned boxes across a break, text shaping and bidi, and layout loops that never end.
The advisories add: fetches and file reads through every URL a document can name, a second loader that
skips the first one's check, scheme spellings, XML entities in SVG, image bombs, string injection into
the PDF's own syntax, and active content in links.

**Where the engine already stands.** No I/O exists in the component, every asset goes through
`Assets::resolve`, links are filtered to `http`, `https`, `mailto` and `#id`, and the output is scanned
for script and launch actions, so the whole fetch and file-read family is closed by design
(`rule:core-classes/pdf-render-has-no-io`). Tables already repeat `thead` and `tfoot`, close their
borders at a break, never leave a header alone and keep rowspan groups whole. Orphans, widows,
`break-*: avoid`, named pages, running elements, `target-counter()` with a 5-pass cap, bookmarks,
byte-equal output and 10,000 nested divs are pinned.

**What the audit found wrong.** The rule `rule:core-classes/pdf-css-subset` says nothing is dropped
silently, and several things are: `overflow` (no layout code reads it), `visibility`,
`text-align-last`, `<col>` widths, `break-*` on `tr` and `tbody`, `@page :nth()`, `counter-reset:
page`, `@media` features, `<ol type>` and `reversed`. Beyond those:
- `<img>` is always a block (`extensions/pdf/src/layout/inline.rs:1676`), so it cannot sit in a line,
  take `vertical-align` or float inside a link;
- `font-size: 0` divides glyph advances by zero (`extensions/pdf/src/write.rs:529`);
- negative margins do nothing (`pending.max(margin)`, `extensions/pdf/src/layout/block.rs:766`);
- `var()` expansion doubles at each level up to depth 32 (`extensions/pdf/src/css/value.rs:277`);
- `1e308px` becomes infinity and is used unchecked;
- page parity ignores `direction`, a forced break drops the next margin, and a box that fills the page
  exactly adds a page for its bottom padding;
- table layout is quadratic in rows, and margin-box lookups are quadratic in pages.

Each stage below fixes one area, and stages 8 and 9 pin with a test every class the audit found
handled but untested.

## Standing decisions

1. **Accepted but ignored is a silent drop, and a bug.** Every property, value, attribute or at-rule
   the cascade accepts is either applied or listed in `Rendered::dropped`. Where this goal does not
   implement one, it is removed from `extensions/pdf/subset.json` so the cascade drops and reports it.
   Safe default: report.
2. **`<img>` and inline `<svg>` are inline replaced boxes**, as CSS says. `display: block` makes them
   blocks. The module doc of `extensions/pdf/src/layout/image.rs` changes with them. Cost: one parley
   inline box per image, no new memory per request.
3. **No new limits.** A pixel bomb, a `var()` chain or many `@font-face` rules end at the request's
   memory ceiling and CPU deadline (`no-overshoot` floor). The fix is to make the work linear or
   memoised where it is not, and an attack proves the ceiling holds. A non-finite length is an
   invalid value, dropped and reported. That is a value check, not a limit.
4. **Break points that stay as they are:** a `break-inside: avoid` box taller than a page splits where
   it stands (pinned at `extensions/pdf/tests/paginate.rs:348`), and a rowspan group is kept whole
   unless it is taller than a page (`rule:core-classes/pdf-tables-across-pages`).
5. **Page parity:** page 1 is a right page in a left-to-right document and a left page in a
   right-to-left one, read from the root's `direction`. `recto` and `verso` follow. When a
   `break-after` and a `break-before` meet, the later element's side wins.
6. **Margins at a break**, CSS Fragmentation §5.2: an unforced break truncates the adjoining margin to
   0, and a forced break keeps it.
7. **Fit tests use a tolerance of 0.01 pt**, the one `extensions/pdf/src/layout/table.rs` already uses,
   in every `>` that decides whether something fits.
8. **Collapsed borders** resolve by CSS 2.1 §17.6.2.1: `hidden` wins, then the wider border, then the
   style order double, solid, dashed, dotted, ridge, outset, groove, inset, then the owner order cell,
   row, row group, column, column group, table.
9. **What is implemented and what is reported:**
   - Implemented: `overflow: hidden` clipping, `visibility: hidden` and `collapse`, negative margins,
     `counter-reset: page`, `<ol type>` and `reversed`, `<col>` and `<colgroup>` widths,
     `break-before/after: page` on `tr` and `tbody`, EXIF orientation of a JPEG, a header and footer
     left off a page where a row cannot fit with them, and `@media` types (`all`, `print`) and the
     `width`, `height` and `orientation` features, evaluated against the page size.
   - Implemented if parley takes them per span, else reported: `white-space` and `line-height` on an
     inline element, and `text-align-last`. The session checks the installed parley 0.12's
     `StyleProperty` and `AlignmentOptions` first (AGENTS.md rule 13).
   - Reported: `@page` pseudo-classes other than `:first`, `:left`, `:right` and `:blank`, `break-*` on
     floats and flex items, an `@media` feature outside the list above, and a linked stylesheet that is
     not UTF-8.
10. **EXIF orientation** is read by hand from the JPEG's APP1 segment (the orientation tag only), with
    no new dependency.
11. **Selector matching** buckets rules by the rightmost compound's id, class or tag, as browsers do, so
    a rule is tested only against elements that can match it.
12. **Tests.** Every class is a Rust test that asserts structure, not pixels: page count, box
    positions, text per page, and the `dropped` and `overflows` lists. Each stage writes its own test
    file, named in its check, so writers never share a file. Cost tests count work (decodes, rule
    tests, rows visited), never time. Security classes also get an attack under
    `tests/hostile/novis/Pdf-Document/`, held to AGENTS.md § *Text an end user reads*. Test text uses
    neutral content (`Shop`, `example.com`, lorem).
13. **Lanes.** Stages 1 to 7 share `inline.rs`, `block.rs` and `text.rs`, so they run in order. Stages
    8 and 9 write only new test files and can run as side goals once stage 7 is done.
14. **An image's percent height** counts against the page's content height, the only height a
    document has, and so does a percent `min-height` and `max-height`. A percent width counts
    against the containing block. A size whose aspect ratio is free (one or no size given) is
    limited by the CSS 2.1 §10.4 table, and with both sizes given each side is limited alone. Cost:
    nothing more.

## Stages

### 1. hostile CSS, images, SVG and link text never break a render, and each bomb ends at the request's ceiling

**Does:** closes the robustness holes the audit found, each with a test in the new
`extensions/pdf/tests/hostile_input.rs` and one attack file.

- [x] A length, a font size or a page size that is not finite after parsing or arithmetic is an
  invalid value: dropped and reported, and an `@page size` falls back to A4. Check at the cascade's
  value conversion, and at `size()` and `margin_rule`. `extensions/pdf/src/css/value.rs:1`,
  `extensions/pdf/src/paginate/mod.rs:454`
- [x] `var()` resolves each custom property once per element and reuses it, so a chain where each
  property names the next one twice costs linear work. The output size is left to the memory ceiling
  (decision 3). `extensions/pdf/src/css/value.rs:228`
- [x] A glyph run of size 0 is not drawn, and nothing in `write.rs` divides by a size of 0.
  `extensions/pdf/src/write.rs:524`
- [x] Tests for a link URI and a title, author and subject with `( ) \ % & < >` and `é`: the file
  parses back with `read.rs`, the text is unchanged, and PDF/A still validates.
  `extensions/pdf/src/write.rs:457`, `extensions/pdf/src/write.rs:667`
- [x] A test that `file%3A//x` and `HTTP://x` are missing assets, and an SVG asset with a DTD of
  nested entities, an external entity and a `<use>` cycle renders or throws `Parse`.
  `extensions/pdf/src/assets.rs:102`
- [x] Write the seven tests the `hostile-input` check names. `extensions/pdf/tests/hostile_input.rs:1`
- [x] Attack `tests/hostile/novis/Pdf-Document/render/02-values-that-grow-without-end.nvs`: `1e308px`
  widths, a `var()` doubling chain, a PNG that declares 60000 × 60000 pixels, and 10,000
  `@font-face` rules. Follow `tests/hostile/README.md` and the render directory's `nvs.toml`.
- [x] Run the `hostile-input` check's command once: green.

### 2. no part of a render grows faster than the document: tables, margin-box lookups, repeated images and selector matching are linear

**Does:** removes the four quadratic paths, with work-count tests in the new
`extensions/pdf/tests/cost.rs`, modelled on the depth-cost check at `extensions/pdf/tests/layout.rs:2585`.

- [x] Table layout keeps a running suffix sum of piece heights, so `rows_from` and `fits_rest` stop
  re-summing. `extensions/pdf/src/layout/table.rs:547`, `extensions/pdf/src/layout/table.rs:623`
  - [x] `layout::Work` on `Laid`, carried through both `Cx` passes
  - [x] Suffix sums of height and row count, every piece read counted in `table_reads`
  - [x] `cost.rs` `a_table_ten_times_longer_costs_ten_times_the_work`, run once: green
- [x] Named strings, counters, running elements and marks are indexed by page once, after layout, so
  each margin box reads its page's entries directly. `extensions/pdf/src/layout/margin.rs:85`,
  `extensions/pdf/src/layout/margin.rs:149`, `extensions/pdf/src/layout/margin.rs:357`
  - [x] `margin::Index` by name and page, `string`, `counter` and `pick` read through it
  - [x] `facts` sorts marks onto pages in one pass
  - [x] `cost.rs` `a_string_set_on_every_page_…`, run once: green
- [x] Images are cached by asset reference for the render: one resolve and one decode per distinct
  asset. `extensions/pdf/src/layout/image.rs:70`, `extensions/pdf/src/layout/image.rs:192`
  - [x] `Builder::load` keeps each reference's index in `images`, for `<img>` and backgrounds
  - [x] the counts reach `Laid::work` through `blocks`' result
  - [x] `cost.rs` `one_image_used_a_thousand_times_…`, run once: green
- [x] Rules are bucketed by the rightmost compound's id, class or tag (decision 11).
  `extensions/pdf/src/css/mod.rs:812`
  - [x] `Buckets` files each selector once, by the compound before a pseudo-element too
  - [x] `Cascade::matched` tests only the candidates, counted in `Styles::rule_tests`
- [x] Write the four tests the `linear-cost` check names. Each compares the work at N and 10N.
  `extensions/pdf/tests/cost.rs:1`
- [x] Run the `linear-cost` check's command once: green.

### 3. an <img> and an inline <svg> are inline boxes in their line, with vertical-align, float, max and min sizes and EXIF orientation

**Does:** makes images inline replaced boxes (decision 2), with tests in the new
`extensions/pdf/tests/images.rs`.

- [x] An `<img>` or inline `<svg>` whose `display` is inline becomes a parley inline box sized like the
  picture, placed in its line. `display: block` keeps today's path, and `float` goes through the float
  path, also inside `<a>`. `extensions/pdf/src/layout/inline.rs:1670`
  - [x] `Builder::replaced` opens an atom with the image's style, places the picture in it and
    sets the box's width and height to the picture's
  - [x] a float closes through `close_float`, a flex item and any other `display` keep the block path
- [x] The picture is drawn at its inline box's position and keeps its `Figure` tag and alt text.
  `extensions/pdf/src/layout/image.rs:142`, `extensions/pdf/src/layout/text.rs:493`
- [x] Image sizing reads `max-width`, `max-height`, `min-width`, `min-height` and a percent height,
  and an image taller than the page shrinks to the page. `extensions/pdf/src/layout/image.rs:104`
  - [x] `constrain` is CSS 2.1 §10.4's table, `bound` limits each side when both are given
  - [x] percent heights count against the page's content height (decision 14)
- [x] EXIF orientation of a JPEG turns the picture (decision 10). `extensions/pdf/src/layout/image.rs:164`
  - [x] `Turn::of_jpeg` reads the APP1 orientation tag, `Picture::Raster` carries it
  - [x] `size` swaps for 5 to 8, and `write.rs`'s `draw_picture` draws through `Turn::matrix`
- [x] Existing tests that expect an image as a block are updated where the markup is inline, and the
  module doc says images are inline boxes. `extensions/pdf/tests/layout.rs:830`,
  `extensions/pdf/src/layout/image.rs:1`
  - [x] `layout`, `paginate` and `render` run once: green, so no test needed a change
- [x] Write the seven tests the `inline-images` check names. `extensions/pdf/tests/images.rs:1`
- [x] Run the `inline-images` check's command once: green.

### 4. page breaks fall where CSS Fragmentation puts them: parity by direction, kept margins, no padding-only page, floats after a break, tall boxes that continue

**Does:** fixes the pagination findings, with tests in the new `extensions/pdf/tests/breaks.rs`.

- [x] Page parity reads the root's `direction`, and `recto` and `verso` follow it (decision 5).
  `extensions/pdf/src/layout/block.rs:100`, `extensions/pdf/src/layout/block.rs:483`,
  `extensions/pdf/src/paginate/mod.rs:7`
- [x] A forced break keeps the next box's top margin (decision 6). `extensions/pdf/src/layout/block.rs:566`
- [x] Every fit test uses the 0.01 pt tolerance (decision 7), and `close()` does not start a page for
  the bottom padding and border alone. `extensions/pdf/src/layout/block.rs:717`,
  `extensions/pdf/src/layout/block.rs:835`, `extensions/pdf/src/layout/text.rs:1399`
- [x] `Flow::float` takes a pending forced break before it places the float.
  `extensions/pdf/src/layout/block.rs:652`
- [x] A `break-after: avoid` heading before a block taller than a page joins only that block's first
  lines (its orphans), not the whole block. `extensions/pdf/src/layout/text.rs:1467`
- [x] A fixed-height box taller than a page continues on the next page with its background sliced,
  and a flex band taller than a page splits between the lines of its items.
  `extensions/pdf/src/layout/block.rs:826`, `extensions/pdf/src/layout/flex.rs:339`
- [x] Write the nine tests the `breaks` check names. `extensions/pdf/tests/breaks.rs:1`
  - [x] the six for parity, the kept margin, the bottom edge, the tolerance and the float
  - [x] `a_heading_before_a_block_taller_than_a_page_moves_with_its_first_lines`
  - [x] `a_fixed_height_box_taller_than_a_page_continues_on_the_next_page`
  - [x] `a_flex_line_taller_than_a_page_splits_between_its_items_lines`
- [x] Run the `breaks` check's command once: green.

### 5. tables read col widths, row breaks and HTML span numbers, resolve collapsed borders by CSS 2.1, and run right to left

**Does:** fixes the table findings, with tests added to `extensions/pdf/tests/tables.rs`.

- [ ] `<col>` and `<colgroup>` widths, with `span`, set the column widths.
  `extensions/pdf/src/layout/inline.rs:1061`, `extensions/pdf/src/layout/table.rs:925`
- [ ] `break-before: page` and `break-after: page` on `tr` and `tbody` start a new page.
  `extensions/pdf/src/layout/table.rs:108`
- [ ] `rowspan="0"` spans to the end of its row group, and span attributes are read by HTML's rules for
  non-negative integers (`2x` is 2). `extensions/pdf/src/layout/inline.rs:1047`
- [ ] Collapsed borders resolve by decision 8, at a break and under a repeated header too.
  `extensions/pdf/src/layout/table.rs:58`
- [ ] A header and footer that cannot fit with a row are left off that page.
  `extensions/pdf/src/layout/table.rs:597`, `extensions/pdf/src/layout/table.rs:636`
- [ ] A table and a flex row with `direction: rtl` run from the right. `extensions/pdf/src/layout/table.rs:1`,
  `extensions/pdf/src/layout/flex.rs:61`
- [ ] The rule's *On disk* paragraph names the new behaviours.
  `docs/rules/core-classes/pdf-tables-across-pages.md:1`, then `bun nv rules --render` and
  `bun nv rules --check`.
- [ ] Write the eight tests the `tables` check names. `extensions/pdf/tests/tables.rs:764`
- [ ] Run the `tables` check's command once: green.

### 6. every text and box property the subset accepts is applied: overflow clips, visibility hides, margins go negative, span-level white-space and line-height, list types

**Does:** applies or reports each property the cascade accepts and layout ignores (decisions 1 and 9),
with tests in the new `extensions/pdf/tests/boxes.rs`.

- [ ] `overflow: hidden` (and `overflow-x`/`overflow-y`) clips a box's content to its padding box, and
  clipped content adds no page. `extensions/pdf/src/layout/block.rs:826`, `extensions/pdf/src/layout/boxes.rs:1038`
- [ ] `visibility: hidden` draws nothing and keeps the space, `collapse` on a table row takes no space.
  `extensions/pdf/src/css/properties.rs:330`
- [ ] Margins collapse to the largest positive plus the most negative, so a negative margin pulls the
  box up. `extensions/pdf/src/layout/block.rs:766`, `extensions/pdf/src/layout/block.rs:911`
- [ ] `white-space` and `line-height` on an inline element, and `text-align-last`: applied per span if
  parley 0.12 takes them, else removed from `subset.json` (decision 9).
  `extensions/pdf/src/layout/text.rs:478`, `extensions/pdf/src/layout/inline.rs:135`
- [ ] `<ol type>` maps to `list-style-type` as a presentational hint, and `reversed` counts down.
  `extensions/pdf/src/layout/inline.rs:274`
- [ ] Each `subset.json` description that changed says what the property now does.
  `extensions/pdf/subset.json:87`
- [ ] Write the eight tests the `boxes` check names. `extensions/pdf/tests/boxes.rs:1`
- [ ] Run the `boxes` check's command once: green.

### 7. page counters restart, unknown page selectors and @media features are reported, and margin boxes report their overflow

**Does:** finishes the page-level findings, with tests in the new `extensions/pdf/tests/pages.rs`.

- [ ] `counter-reset: page N` on an element restarts the page number on the page that element starts.
  `counter(pages)` stays the total. `extensions/pdf/src/layout/margin.rs:187`
- [ ] An `@page` pseudo-class outside `:first`, `:left`, `:right` and `:blank` is reported in
  `dropped`. `extensions/pdf/src/paginate/mod.rs:147`, `extensions/pdf/src/css/mod.rs:421`
- [ ] `@media` evaluates its type and the `width`, `height` and `orientation` features against the
  page size, and reports any other feature. `extensions/pdf/src/css/mod.rs:318`
- [ ] A linked stylesheet that is not UTF-8 is reported. `extensions/pdf/src/css/mod.rs:674`
- [ ] A CSS margin box or `element()` taller than its margin is listed in `Rendered::overflows`.
  `extensions/pdf/src/layout/margin.rs:9`
- [ ] Corner margin boxes take the css-page-3 default alignment. Read the spec's table first; the audit
  remembered top-left and bottom-left as right, and top-right and bottom-right as left.
  `extensions/pdf/src/paginate/mod.rs:264`
- [ ] Write the six tests the `pages` check names. `extensions/pdf/tests/pages.rs:1`
- [ ] Run the `pages` check's command once: green.

### 8. the pagination and table classes the engine already handles are pinned by tests

**Does:** writes one test per class the audit found handled but untested. A red test is a bug: fix its
cause in this stage, or, if the fix does not fit, add it to § *Backlog* with its test's name.

**Split:** two writing subagents side by side, one per file, each given its half of the list below and
the helpers at `extensions/pdf/tests/paginate.rs:8`. Neither builds; the session builds and runs the
check once both end.

- [ ] `extensions/pdf/tests/edge_pagination.rs:1`, the first thirteen tests the `pinned-pagination`
  check names. The behaviour each pins: adjacent breaks merge (`block.rs:753`), a trailing break adds
  no page (`block.rs:539`), a break at the page top adds none (`block.rs:480`), a break inside a fixed
  element is ignored (`block.rs:477`), a break passes up through wrappers (`block.rs:809`), the margin
  drops inside a flow root, a three-box avoid chain (`text.rs:1448`), the orphans and widows matrix
  (room for 4 lines, 7 lines: `2/2` → 4+3, `orphans:5` → 0+7, `widows:4` → 3+4, `4/4` → 0+7), text
  split across pages joins back to the source, a marker on the first fragment only, numbering and
  counters across pages, fixed content on forced-break pages, and the blank page between named groups.
- [ ] `extensions/pdf/tests/edge_tables.rs:1`, the last seven tests the check names: a two-row `thead`
  repeats whole, an empty table, an empty `tr` and a zero-width table do not panic, `table-layout:
  fixed` and percentages over 100%, a rowspan cell's `vertical-align`, a nested table across pages,
  a float inside a cell, and a link in a repeated header.
- [ ] Run the `pinned-pagination` check's command once: green.

### 9. the text, image, CSS and output classes the engine already handles are pinned by tests

**Does:** the same as stage 8, for the rest of the audit.

**Split:** two writing subagents side by side, one per file. Neither builds.

- [ ] `extensions/pdf/tests/edge_text.rs:1`, the first ten tests the `pinned-text-output` check names:
  a font listed first draws its digits (`fonts.rs:188`), kinsoku through ICU, `break-word` after an
  inline boundary, `<pre>`'s first newline and leading spaces, an inline-block ending in a space,
  entities, `!important` in a shorthand (`css/mod.rs:602`), `border-box` and percent vertical
  margins, a float wider than its container (`block.rs:669`), and markers past Z and past 3999
  (`inline.rs:301`).
- [ ] `extensions/pdf/tests/edge_output.rs:1`, the last eight tests: aspect ratio from one dimension,
  CMYK JPEG and 16-bit and interlaced PNG, `cover` and `contain` (`boxes.rs:163`), a wrapped link
  (`text.rs:1097`), one structure element across pages (`write.rs:278`), a merged rotated page
  (`merge.rs:74`), a password with an escaped title (`encrypt.rs:370`), and an invoice XML that
  contains `<?xpacket begin=` (`factur_x.rs:166`).
- [ ] Run the `pinned-text-output` check's command once: green.

### 10. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 4 are done. `hostile-input` is green (7 of 7), `linear-cost` (4 of 4),
`inline-images` (7 of 7) and `breaks` (9 of 9).

- A group joined by `break-after: avoid` that is taller than the next page is kept by its head
  only: `Cx::head` (`extensions/pdf/src/layout/text.rs:580`) turns the last joined unit into its
  opening boxes, with no bottom edge, and a spacer as tall as its first paragraph's `orphans`
  lines. `join` returns where that unit starts in the group.
- `Flow::close` places the rest of a `height` or `min-height` a page's room at a time
  (`extensions/pdf/src/layout/block.rs:895`), so the box is sliced over every page.
- A flex band taller than the next page goes through `Cx::split_band`
  (`extensions/pdf/src/layout/flex.rs:378`): each item is laid out again with `Cx::down` over a
  `Flow::paged` of the room left, then whole pages, as `table.rs` does for a tall row.
- `block::FIT` (0.01 pt) is the tolerance in every fit test; a new one in stage 5 uses it too.
- The sweep's three reds were stale again: `env-secret-proofs` is defined nowhere and has no log,
  and nightly 41813182715 is the run the calm-editor handoff already traced to fixed commits.

### Next

Stage 5, the tables, in item order. Start with the span attributes and `<col>`, which share
`extensions/pdf/src/layout/inline.rs:1073` (`count("colspan", …)` and `count("rowspan", …)`):
read each by HTML's rules for non-negative integers (leading digits, `2x` is 2), and let
`rowspan="0"` run to the end of its row group. Then `<col>` and `<colgroup>` widths into the
column computation (`extensions/pdf/src/layout/table.rs:956` `column_ranges`), then row and
row-group `break-before/after: page` (the piece loop, `table.rs:655`), then the collapsed-border
resolution of decision 8, the header and footer left off a page they cannot fit on with a row,
and right-to-left tables and flex rows. The eight tests go at the end of
`extensions/pdf/tests/tables.rs` (after line 780).

Tests that may go red at stage 10: `crates/nvs-ext/tests/pdf.rs` was not run after images became
inline boxes; a test there that places an `<img>` in running text and reads its `y` as a block's
now sees it on the line. Any test asserting an empty `dropped` list with an `@page size`
that does not read (`paginate.rs:133` asserts only the page height). Any test that counted
`laid.images.len()` for a document that draws one asset twice now sees one. The margin, string and
counter tests in `paginate.rs` were not run after `margin::Index`; their order rules are unchanged.
A test that reads a box's top after a forced break now sees its top margin kept (`paginate.rs`,
`layout.rs`, `crates/nvs-ext/tests/pdf.rs`), and one that expected a page holding only a box's
bottom padding now sees one page fewer. A test with a fixed-height box that starts low on a page
and does not fit under it now sees the box split there, not moved whole to the next page. A test
with a flex row taller than a page now sees its items split and not stretched.

### Backlog
