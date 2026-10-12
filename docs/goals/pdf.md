---
looper: 1
title: "`Novis\\Pdf` is built into every binary"
summary: "The third component built into every `nvs` binary, and the image component's second wave beside it."
context: [crates/nvs-ext/build.rs, crates/nvs-ext/src/builtin.rs, crates/nvs-ext/src/load.rs, extensions/image/src/lib.rs, wit/image.wit]
stages:
  - id: 2
    title: "the fragments, `wit/pdf.wit` and the font licence"
    status: done
    extra:
      context:
        rules: [core-api/tier-roster, core-classes/pdf-render-has-no-io, core-classes/pdf-css-subset, core-classes/pdf-output-is-inert, core-classes/pdf-one-engine, core-classes/pdf-page-is-an-image-source, core-classes/pdf-decode-refusals, packaging/pdf-decoding-ships-in-the-image-package, packaging/a-value-crosses-as-its-wit-type, core-api/options-bag, core-api/one-paradigm-per-operation, core-classes/html-template, enums/an-enum-lives-in-the-class-that-uses-it]
        shapes: [A rule fragment]
  - id: 3
    title: SVG and PDF open as images
    status: done
    extra:
      context:
        rules: [core-classes/image-format-roster, core-classes/image-pixel-cap, core-classes/pdf-page-is-an-image-source, core-classes/pdf-decode-refusals]
        shapes: ["A `.nvst` test case"]
  - id: 4
    title: "shaping and barcodes in `Novis\\Image`"
    status: done
    extra:
      context:
        rules: [core-classes/image-one-entry-point-per-job]
        shapes: ["A `.nvst` test case"]
  - id: 5
    title: the PDF component is built in
    status: done
    extra:
      context:
        rules: [core-classes/pdf-output-is-inert, core-classes/html-parsing, packaging/a-wasm-module-cache-reuses-the-artifact-cache, packaging/the-first-party-components-are-built-in, packaging/a-prebuilt-wasm-library-is-rebuilt-in-ci]
        shapes: ["A `.nvst` test case"]
  - id: 6
    title: the cascade
    status: done
    extra:
      context:
        rules: [core-classes/pdf-css-subset]
        shapes: []
  - id: 7
    title: "text, blocks, lists and images"
    status: done
    extra:
      context:
        rules: [core-classes/pdf-css-subset]
        shapes: []
  - id: 8
    title: "flex, grid, floats and positioning"
    status: done
    extra:
      context:
        rules: []
        shapes: []
  - id: 9
    title: pagination
    status: done
    extra:
      context:
        rules: [core-classes/pdf-output-is-inert]
        shapes: []
  - id: 10
    title: header and footer functions
    status: done
    extra:
      context:
        playbook: ["Writing Novis > in extensions/intl/nvs/Collator.nvs"]
        rules: [packaging/a-fresh-instance-per-request, core-classes/html-template]
        shapes: ["A `.nvst` test case"]
  - id: 11
    title: tables across pages
    status: done
    extra:
      context:
        rules: []
        shapes: ["A `.nvst` test case"]
  - id: 12
    title: "standards, Factur-X and passwords"
    status: done
    extra:
      context:
        rules: [core-classes/pdf-output-is-inert]
        shapes: ["A `.nvst` test case"]
  - id: 13
    title: "existing PDFs: placing, merging and reading"
    status: done
    extra:
      context:
        rules: [core-classes/pdf-decode-refusals]
        shapes: ["A `.nvst` test case"]
  - id: 14
    title: "the growth, the benches and a bundle"
    status: done
    extra:
      context:
        rules: [packaging/a-nvsx-dependency-embeds-in-the-same-payload, core-classes/pdf-page-is-an-image-source]
        shapes: []
  - id: 15
    title: completion and hover in the editor
    status: open
    extra:
      context:
        rules: [ide/a-template-region-gets-the-editors-services-and-formatter, ide/completion-is-asked-where-a-spelling-ends]
        shapes: []
  - id: 16
    title: the feature proofs
    status: open
    extra:
      context:
        playbook: ["Tooling > `bun nv proofs --verify --group`*"]
        rules: [testing/feature-proofs, core-api/reference-card, testing/hostile-case-contract]
        shapes: ["Feature proofs — an example, an attack, a bench, a `covers:` marker"]
checks:
  - id: pdf-the-record-creates-its-fragments
    stage: 2
    name: "rulebook: Stage 2 created the `Novis\\Pdf` fragments"
    run: [bun, nv, brief, --where, pdf]
    output_ordered: ["rule:core-classes/pdf-css-subset-table", "rule:core-classes/pdf-tables-across-pages", "rule:core-classes/pdf-header-is-a-page-function", "rule:core-classes/pdf-factur-x-is-the-one-attachment", "rule:core-classes/pdf-reader-and-merge", "rule:core-classes/pdf-default-fonts"]
  - id: image-the-record-creates-the-barcode-roster
    stage: 2
    name: "rulebook: Stage 2 created the image component's barcode roster"
    run: [bun, nv, brief, --where, barcode]
    output_ordered: ["rule:core-classes/image-barcode-roster"]
  - id: nvs-stdlib-the-pdf-interface-parses-against-the-world
    stage: 2
    name: "nvs-stdlib (`wit/pdf.wit` parses against the world in its types alone, and carries the second pass for header and footer markup)"
    run: [cargo, test, --test, ext_world]
    tests: [the_pdf_interface_parses_against_the_world_in_wit_types_alone, the_pdf_render_export_carries_a_second_pass_for_page_markup]
  - id: nvs-stdlib-the-image-codec-exports-a-barcode
    stage: 2
    name: "nvs-stdlib (`wit/image.wit` exports `barcode` as the ninth entry point, and every `Novis\\Image\\Barcode` member runs in it)"
    run: [cargo, test, --test, ext_world]
    tests: [the_image_codec_exports_exactly_the_nine_entry_points, every_image_builder_member_maps_to_an_export_a_plan_step_or_novis_source]
  - id: deny-the-image-component-graph-passes
    stage: 2
    name: "cargo deny: the image component's own `wasm32-wasip2` graph passes `deny.toml`"
    run: [cargo, deny, --manifest-path, extensions/image/Cargo.toml, --target, wasm32-wasip2, --locked, check]
    output_ordered: ["advisories ok, bans ok, licenses ok, sources ok"]
  - id: deny-the-intl-component-graph-passes
    stage: 2
    name: "cargo deny: the intl component's own `wasm32-wasip2` graph passes `deny.toml`"
    run: [cargo, deny, --manifest-path, extensions/intl/Cargo.toml, --target, wasm32-wasip2, --locked, check]
    output_ordered: ["advisories ok, bans ok, licenses ok, sources ok"]
  - id: nvs-ext-svg-and-pdf-decode-as-images
    stage: 3
    name: "nvs-ext (SVG decodes with no resource resolved, a PDF page rasterises at its dpi under the pixel cap, `info` counts pages without a pixel, and the bomb corpus ends in a throw or a contained trap)"
    run: [cargo, test, --test, image_decode]
    tests: [an_svg_decodes_and_an_external_href_throws, a_pdf_page_rasterises_at_the_requested_dpi, info_counts_pdf_pages_and_allocates_no_pixel, a_pdf_page_past_the_end_throws_naming_the_count, a_pdf_page_over_the_pixel_cap_is_refused_before_a_buffer, an_encrypted_pdf_throws_a_parse_error, an_unembedded_standard_font_renders_with_its_substitute, the_pdf_bomb_corpus_ends_in_a_throw_or_a_contained_trap, two_rasterisations_of_a_pdf_page_are_byte_equal]
  - id: image-svg-and-pdf-open-as-images
    stage: 3
    name: "`Image::open` opens an SVG and a chosen page of a PDF, and a page past the end throws"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/image-an-svg-opens-as-an-image.nvst, --case, tests/conformance/novis/image-a-pdf-page-opens-as-an-image.nvst]
    tests: [tests/conformance/novis/image-an-svg-opens-as-an-image.nvst, tests/conformance/novis/image-a-pdf-page-opens-as-an-image.nvst]
  - id: nvs-ext-text-shapes-every-script
    stage: 4
    name: "nvs-ext (`Image::text` shapes Arabic, Hebrew and Devanagari through `harfrust`, and Latin text keeps its kerning)"
    run: [cargo, test, --test, image_analysis]
    tests: [text_shapes_arabic_right_to_left_with_joined_letters, text_shapes_devanagari_with_its_conjuncts, latin_text_keeps_its_kerning_under_the_shaper]
  - id: nvs-ext-barcodes-read-back
    stage: 4
    name: "nvs-ext (every 1D symbology and DataMatrix reads back as the data written, QR renders as SVG, the EPC payload is the GiroCode layout, and a wrong check digit throws)"
    run: [cargo, test, --test, image_barcode]
    tests: [every_1d_symbology_reads_back_as_the_data_written, a_datamatrix_reads_back_as_the_data_written, a_qr_code_and_a_barcode_render_as_svg, an_epc_payload_is_the_girocode_layout, a_wrong_check_digit_throws_naming_it]
  - id: image-a-barcode-renders
    stage: 4
    name: "`Novis\\Image\\Barcode` renders a Code 128 and an EAN-13 as SVG, and a wrong check digit throws"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/image-a-barcode-renders-as-svg.nvst]
    tests: [tests/conformance/novis/image-a-barcode-renders-as-svg.nvst]
  - id: nvs-ext-the-pdf-component-is-built-in
    stage: 5
    name: "nvs-ext (the PDF component is built in, renders with no configuration and no grant, resolves only the asset map and `data:`, writes byte-equal inert output, and holds the HTML element cap)"
    run: [cargo, test, --test, pdf]
    tests: [the_pdf_component_is_built_in_and_answers_with_no_configuration, a_render_needs_no_capability_grant, a_paragraph_renders_with_the_default_fonts, a_file_or_http_reference_throws_naming_it, a_name_missing_from_the_asset_map_throws, a_data_uri_image_renders, two_renders_of_equal_input_are_byte_equal, the_output_has_no_javascript_launch_or_open_action, html_nested_past_the_element_cap_flattens_as_core_html_does]
  - id: pdf-renders-with-no-configuration
    stage: 5
    name: "`Novis\\Pdf` renders from a binary with no `nvs.toml`, and a missing asset throws"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/pdf-renders-with-no-configuration.nvst, --case, tests/conformance/novis/pdf-a-missing-asset-throws.nvst]
    tests: [tests/conformance/novis/pdf-renders-with-no-configuration.nvst, tests/conformance/novis/pdf-a-missing-asset-throws.nvst]
  - id: pdf-the-cascade-and-the-dropped-list
    stage: 6
    name: "extensions/pdf (the cascade decides computed styles, every property the subset table lists parses, and every other declaration is dropped and reported)"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, css]
    tests: [specificity_inheritance_and_important_decide_the_computed_style, var_and_calc_resolve, media_print_applies_and_screen_does_not, every_property_in_the_subset_table_parses, a_declaration_outside_the_subset_is_dropped_and_reported_with_its_position, a_clean_document_reports_nothing_dropped]
  - id: pdf-text-blocks-lists-and-images
    stage: 7
    name: "extensions/pdf (text wraps, justifies, hyphenates and runs right to left, CJK breaks between ideographs, a missing glyph is reported, and lists, boxes, images and SVG lay out)"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, layout]
    tests: [text_wraps_and_justifies_inside_its_box, arabic_and_hebrew_lay_out_right_to_left, cjk_breaks_between_ideographs, hyphens_auto_breaks_by_the_language, a_missing_glyph_is_drawn_as_the_empty_box_and_reported_with_its_chain, a_font_face_from_the_asset_map_is_used_and_subset, arial_helvetica_times_and_courier_map_to_liberation, lists_and_counters_number_their_items, borders_radius_backgrounds_and_gradients_lay_out, an_inline_svg_is_vector_content_in_the_output]
  - id: pdf-flex-grid-floats-and-positioning
    stage: 8
    name: "extensions/pdf (flex and grid place their items, a float wraps the text beside it, absolute and fixed position as CSS says, and a rotation is drawn)"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, layout]
    tests: [flex_lays_out_rows_and_columns, grid_places_items_in_tracks, flex_and_grid_break_only_between_items_and_rows, a_float_right_wraps_the_text_beside_it, absolute_positions_against_its_containing_block, fixed_repeats_on_every_page, a_rotated_transform_is_drawn_rotated]
  - id: pdf-pagination
    stage: 9
    name: "extensions/pdf (`@page` sizes and margins, margin boxes print page X of Y, running elements repeat, breaks and orphans and widows hold, a table of contents settles, and links, bookmarks and metadata are written)"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, paginate]
    tests: [page_size_margins_and_first_left_right_blank, margin_boxes_print_page_x_of_y, a_running_element_repeats_on_every_page, break_before_after_and_inside_avoid_are_kept, orphans_and_widows_hold_lines_together, named_pages_switch_page_size, string_set_carries_the_chapter_title, target_counter_and_leader_build_a_table_of_contents, page_numbers_that_never_settle_throw, headings_become_bookmarks, internal_and_external_links_are_annotations, title_meta_and_lang_become_metadata]
  - id: pdf-page-x-of-y-runs
    stage: 9
    name: "a long document paginates under `@page` and prints its page count"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/pdf-a-long-document-paginates.nvst]
    tests: [tests/conformance/novis/pdf-a-long-document-paginates.nvst]
  - id: nvs-ext-header-and-footer-functions
    stage: 10
    name: "nvs-ext (a header function runs once per page with its number, count and marks, costs two crossings, never moves the body, reports an overflow, and wins over a margin box)"
    run: [cargo, test, --test, pdf]
    tests: [a_header_function_runs_once_per_page_with_its_number_and_count, marks_on_a_page_and_up_to_it_reach_the_function, a_header_function_costs_two_crossings, a_header_never_moves_the_body, a_header_taller_than_its_margin_is_reported, a_function_header_wins_over_a_margin_box]
  - id: pdf-a-footer-carries-a-running-total
    stage: 10
    name: a footer function prints a carried-forward total from the marks of every page so far
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/pdf-a-footer-carries-a-running-total.nvst]
    tests: [tests/conformance/novis/pdf-a-footer-carries-a-running-total.nvst]
  - id: pdf-tables-across-pages
    stage: 11
    name: "extensions/pdf (the eight table rules: stable columns, repeated thead and tfoot with `data-repeat`, rows whole or split between lines, closed borders, no lone header, groups kept together, the caption once, and slots filled in place)"
    run: [cargo, test, --manifest-path, extensions/pdf/Cargo.toml, --test, tables]
    tests: [column_widths_are_the_same_on_every_page, thead_and_tfoot_repeat_on_every_page, data_repeat_first_last_and_continued_rows, a_row_that_fits_is_never_split, a_row_taller_than_a_page_splits_between_lines, borders_are_closed_at_every_break, slice_restores_the_open_edge, a_header_is_never_alone_and_rows_obey_orphans_and_widows, a_tbody_and_a_rowspan_group_stay_together, the_caption_is_on_the_first_page_only, a_slot_in_a_continued_row_is_filled_without_moving_anything]
  - id: pdf-an-invoice-table-carries-forward
    stage: 11
    name: "an invoice table over three pages repeats its header, shows a carried-forward row where it goes on, and its total on the last page"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/pdf-an-invoice-table-carries-forward.nvst]
    tests: [tests/conformance/novis/pdf-an-invoice-table-carries-forward.nvst]
  - id: nvs-ext-standards-factur-x-and-passwords
    stage: 12
    name: "nvs-ext (output is tagged, PDF/A-2b and 3b validate, PDF/UA-1 throws for a missing alt, Factur-X embeds one XML under PDF/A-3b only, and a password encrypts with AES-256 and stays byte-equal)"
    run: [cargo, test, --test, pdf_standards]
    tests: [output_is_tagged_by_default, pdf_a_2b_and_3b_validate, pdf_ua_1_throws_for_an_image_without_alt, factur_x_embeds_one_xml_under_pdf_a_3b, an_invoice_xml_outside_pdf_a_3b_throws, a_malformed_invoice_xml_throws, a_password_encrypts_with_aes_256_and_stays_byte_equal, a_password_with_a_pdf_a_standard_throws]
  - id: pdf-an-e-invoice-renders
    stage: 12
    name: an invoice renders as PDF/A-3b with its Factur-X XML
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/pdf-an-e-invoice-renders-as-pdf-a-3b.nvst]
    tests: [tests/conformance/novis/pdf-an-e-invoice-renders-as-pdf-a-3b.nvst]
  - id: nvs-ext-existing-pdfs
    stage: 13
    name: "nvs-ext (a PDF page is placed as vector content and as a page background, `merge` combines, selects and reorders, `Reader` returns pages, metadata, attachments and text, and an encrypted source throws)"
    run: [cargo, test, --test, pdf_read]
    tests: [a_pdf_page_in_an_img_is_placed_as_vector_content, a_pdf_letterhead_is_the_page_background, merge_combines_selects_and_reorders_pages, reader_counts_pages_and_reads_metadata, reader_returns_the_factur_x_attachment, reader_extracts_the_text_of_a_page, the_text_of_our_own_render_reads_back, an_encrypted_source_throws_in_merge_reader_and_placement]
  - id: pdf-merge-and-read-run
    stage: 13
    name: "two PDFs merge with a page list, and a received invoice's XML and text read back"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/pdf-merge-takes-chosen-pages.nvst, --case, tests/conformance/novis/pdf-reader-returns-an-invoice-xml-and-text.nvst]
    tests: [tests/conformance/novis/pdf-merge-takes-chosen-pages.nvst, tests/conformance/novis/pdf-reader-returns-an-invoice-xml-and-text.nvst]
  - id: nvs-ext-the-recorded-growth-is-the-embedded-size
    stage: 14
    name: "nvs-ext (each component's module doc states its embedded size, within 5% of the bytes the binary carries, the PDF component included)"
    run: [cargo, test, --test, builtin]
    tests: [the_recorded_growth_matches_the_embedded_components, the_pdf_component_is_one_of_three_built_in]
  - id: nvs-cli-a-bundle-calls-all-three-built-in-components
    stage: 14
    name: "nvs-cli (a bundle built with `nvs build --compile` calls `Novis\\Image`, `Novis\\Intl` and `Novis\\Pdf` from a folder with no `nvs.toml`)"
    run: [cargo, test, --test, bundle]
    tests: [a_bundle_calls_all_three_built_in_components]
  - id: vscode-pdf-custom-data-is-current
    stage: 15
    name: "editor: the HTML and CSS custom data are contributed, generated from `extensions/pdf/subset.json`, and complete `data-repeat`'s values"
    run: [npm, --prefix, editors/vscode, run, "test:headless"]
    output_ordered: [the PDF custom data is what the generator writes from the subset table, "data-repeat completes every, first, last and continued", 0 failing]
  - id: dossier-the-whole-roster-owes-nothing-with-novis-pdf
    stage: 16
    name: "proofs: the whole roster owes nothing with `Novis\\Pdf` and the image second wave in it"
    run: [bun, nv, proofs, --gate]
    output_ordered: [nothing owed in the whole roster]
  - id: dossier-novis-pdf-carries-plain-comments
    stage: 16
    name: "proofs: the `Novis\\` programs carry comments inside the plain-comment bounds"
    run: [bun, nv, proofs, --comments, docs/examples/novis, tests/hostile/novis, benches/members/novis]
    output_ordered: [each inside the bounds]
  - id: dossier-novis-pdf-document-owes-nothing
    stage: 16
    name: "proofs: `Novis\\Pdf\\Document` owes nothing, and its examples and attacks run"
    run: [bun, nv, proofs, --verify, --group, "Novis\\Pdf\\Document"]
    output_ordered: [nothing owed in]
  - id: dossier-novis-pdf-page-owes-nothing
    stage: 16
    name: "proofs: `Novis\\Pdf\\Page` owes nothing, and its examples and attacks run"
    run: [bun, nv, proofs, --verify, --group, "Novis\\Pdf\\Page"]
    output_ordered: [nothing owed in]
  - id: dossier-novis-pdf-rendered-owes-nothing
    stage: 16
    name: "proofs: `Novis\\Pdf\\Rendered` owes nothing, and its examples and attacks run"
    run: [bun, nv, proofs, --verify, --group, "Novis\\Pdf\\Rendered"]
    output_ordered: [nothing owed in]
  - id: dossier-novis-pdf-pages-owes-nothing
    stage: 16
    name: "proofs: `Novis\\Pdf\\Pages` owes nothing, and its examples and attacks run"
    run: [bun, nv, proofs, --verify, --group, "Novis\\Pdf\\Pages"]
    output_ordered: [nothing owed in]
  - id: dossier-novis-pdf-reader-owes-nothing
    stage: 16
    name: "proofs: `Novis\\Pdf\\Reader` owes nothing, and its examples and attacks run"
    run: [bun, nv, proofs, --verify, --group, "Novis\\Pdf\\Reader"]
    output_ordered: [nothing owed in]
  - id: dossier-novis-pdf-engine-owes-nothing
    stage: 16
    name: "proofs: `Novis\\Pdf\\Engine` owes nothing, and its examples and attacks run"
    run: [bun, nv, proofs, --verify, --group, "Novis\\Pdf\\Engine"]
    output_ordered: [nothing owed in]
  - id: dossier-novis-image-barcode-owes-nothing
    stage: 16
    name: "proofs: `Novis\\Image\\Barcode` owes nothing, and its examples and attacks run"
    run: [bun, nv, proofs, --verify, --group, "Novis\\Image\\Barcode"]
    output_ordered: [nothing owed in]
  - id: dossier-novis-image-qrcode-owes-nothing
    stage: 16
    name: "proofs: `Novis\\Image\\QrCode` owes nothing with `svg` and `giroCode` in it, and its examples and attacks run"
    run: [bun, nv, proofs, --verify, --group, "Novis\\Image\\QrCode"]
    output_ordered: [nothing owed in]
extra:
  milestone: M17
  files: []
  env:
    valgrind:
      skip: [examples/fatal.nvs, examples/uncaught.nvs, examples/cache-shared-socket.nvs, examples/queue-sqlite.nvs, examples/cache-shared-tls.nvs]
    wsl:
      targetDir: /var/tmp/nvs-target-wsl
  context:
    milestones: [M17]
    plan: [Open now, Blocking]
    playbook: ["Tooling > the end", "Tooling > one call reads", "Tooling > a commit message", "Tooling > a [context]*", "Tooling > another agent", "Tooling > a crate added to a built-in component", "Running things > a new dependency owes three things", "Running things > target/release/nvs.exe is"]
    rules: [packaging/the-first-party-components-are-built-in, packaging/the-boundary-is-the-cost, core-classes/pdf-render-has-no-io, core-classes/pdf-one-engine, core-classes/pdf-css-subset]
    shapes: [A commit message]
---

# `Novis\Pdf` is built into every binary

## Why

### The target

The third component built into every `nvs` binary, and the image component's second wave beside it.
`Novis\Pdf` turns HTML, a documented CSS subset and inline SVG into a paginated, tagged, byte-equal PDF
with no I/O. It places pages of existing PDFs, merges them, reads their text, metadata and attachments,
and writes PDF/A, PDF/UA-1, Factur-X and password-protected files. `Novis\Image` gains SVG and PDF
decoding, real text shaping and barcodes.

```nvs
use Novis\Pdf\{Document, Page, Pages, Reader, Standard};   // names are Stage 2's; this is a sketch

$result = Document::fromHtml(html`<h1>Invoice <?= $number ?></h1><table>...</table>`)
    ->asset('logo.png', $logo)                      // every image, font, stylesheet and PDF is bytes by name
    ->header(fn(Page $p) => $p->number === 1 ? html`` : html`<img src="logo.png"> <?= $p->string('chapter') ?>`)
    ->footer(fn(Page $p) => html`Carried forward: <?= Money::sum($p->marksUpTo('amount')) ?>
                                 Page <?= $p->number ?> of <?= $p->count ?>`)
    ->standard(Standard::PdfA3b)
    ->invoiceXml($facturX)                          // Factur-X: one XML attachment, PDF/A-3 only
    ->render();                                     // ->bytes, ->pageCount, ->dropped, ->missingGlyphs, ->overflows

$all  = Pages::merge([Pages::of($result->bytes), Pages::of($terms, [1, 2])]);
$text = Reader::open($upload)->text(1);
$png  = Image::open($upload, {page: 1, dpi: 150})->resize(...)->encode();
```

Its scope is § *Standing decisions*, which the user settled question by question when the goal was
written, under one rule of theirs: **every common case is in, an edge case is in when it is easy, and a
hard edge case almost nobody needs stays out.** Spreadsheets (M17 Part 3) are not in it.

### Why here

It stands on finished work only: the extension system, the build script and embedding path goals
`ext-image` and `ext-intl` built for two components, `html5ever` with its 512-element cap
(`rule:core-classes/html-parsing`), and ``html`…` `` templates (`rule:core-classes/html-template`). Its
files are `extensions/image`, a new `extensions/pdf`, `wit/` and `nvs-ext`'s build and load tables, and
it shares none of them with goal `ldap` in front of it. Nothing waits for it, so it goes at the end.
The user put it there.

It carries `position: last` because every goal on the chain is pinned there.

### Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands what
makes it untrue, and not before:

- **By Stage 2**, because they state decisions this goal reverses:
  `docs/rules/core-classes/pdf-one-engine.md` (the builder paragraph, the headless-browser backend,
  the crate list); `pdf-output-is-inert.md` ("never … embedded files", the fidelity-backend
  paragraph); `pdf-render-has-no-io.md` ("a scheme … throws", which `data:` now passes);
  `pdf-decode-refusals.md` and `docs/rules/packaging/pdf-decoding-ships-in-the-image-package.md`
  (text extraction, merging and splitting are no longer third-party; the pdfium fallback is gone);
  `docs/plan/m17.md` Parts 1 and 2 (now this goal's, through a `## milestone: M17` wrap section; Part 3
  stays unscheduled).
- **By the stage that lands the behaviour:** every "Not shipped" paragraph of the seven `pdf-*`
  fragments; `docs/rules/core-classes/image-format-roster.md`'s "Not shipped" list and
  `image-one-entry-point-per-job.md:35` ("SVG is not"), at Stage 3;
  `docs/rules/packaging/the-first-party-components-are-built-in.md`'s count of two, at Stage 5.

The search that closes the stage, run after Stage 16:
`grep -rn -i "pdf\|svg\|headless\|fidelity\|two first-party" docs/rules docs/reference docs/plan/m17.md extensions/image/src`,
read line by line. Every hit is true as it stands or rewritten.
`docs/novis.md`, `docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated and
are regenerated, never edited.

### Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check goals `ext-image`,
`ext-image-analysis`, `ext-intl` and `ldap` turned green stays green; every image fixture decodes as it
does today, and `Image::text` on Latin text gives the same pixels until Stage 4 changes the shaper on
purpose, with the fixture's expected output rewritten in that slice and the reason in its commit.

## Standing decisions

- **The user's calls, as instructions.** One goal for the image second wave and `Novis\Pdf`.
  `Novis\Pdf` is the third built-in component, always on, under `Novis\`. The engine is our own
  cascade, layout and pagination over `krilla`, `parley`, `harfrust` and `taffy`. The CSS subset is the
  list in Stages 6 to 9 and 11. Out of it, dropped and reported: footnotes, multi-column,
  `box-shadow`, `text-shadow`, `filter`, masks, `clip-path`, blend modes, 3D transforms, animation,
  transitions, `position: sticky`, vertical writing modes, ruby, subgrid, splitting one flex or grid
  row, `box-decoration-break: clone`, form fields, `<iframe>`, `<video>`, `<canvas>`, and `<script>`,
  which never runs. `data:` URIs are allowed. The default fonts are Liberation Sans, Serif and Mono in
  four styles, under OFL-1.1 admitted for font data. A missing glyph is reported, never thrown.
  Output is always tagged; PDF/A-2b, PDF/A-3b and PDF/UA-1 are options; Factur-X is the one
  attachment. Existing PDFs are placed in HTML and merged. Password protection, reading attachments and
  metadata, and text extraction are in; form filling and signatures are out. Barcodes are 1D,
  DataMatrix, SVG output and the GiroCode helper; PDF417 and Aztec are out. Input is HTML: the builder of
  `rule:core-classes/pdf-one-engine` is dropped. The headless-browser backend is removed from the rules for good. Headers and
  footers are CSS margin boxes and running elements plus the per-page function. The eight table rules
  of Stage 11 are all in. Marks are one attribute, `data-mark="name:value"`. VS Code completes the
  PDF-specific HTML and CSS through custom data generated from the subset table.
- **The goal writer's calls, not confirmed by the user, also standing.** Reading, merging and text
  extraction live in `Novis\Pdf`; only rasterising a page is `Novis\Image`'s. There is no pdfium
  fallback, because pdfium in wasm needs Emscripten and has no WASI path. **No new limit**: a render
  runs under its request's memory cap and deadline, the HTML under the parser's element cap, a raster
  image priced by the memory cap, and a bulk render belongs in `Core\Queue`. Class names in the sketch
  and the checks (`Document`, `Page`, `Rendered`, `Pages`, `Reader`, `Standard`, `Engine` for the
  manifest class, `Novis\Image\Barcode`) are Stage 2's to change; a rename rewrites the checks in
  the same slice.
- **The crates**, each pinned: `krilla` and `krilla-svg` (with the `pdf` feature for placing pages),
  `parley` with `fontique`'s system discovery off, `harfrust`, `skrifa`, `taffy`, `cssparser`,
  `selectors`, `hypher`, `html5ever` through `vendor/html5ever`, `hayro`, `resvg` and `usvg`,
  `barcoders`, `datamatrix`. `cssparser` and `selectors` are MPL-2.0, already on `deny.toml`'s list.
  `fulgur` (the same design, young) and WeasyPrint are references and test comparisons, never
  dependencies. A crate that fails to build for `wasm32-wasip2` is replaced by its nearest pure-Rust
  equivalent, and `core-classes/pdf-one-engine`'s fragment says so.
- **How a session judges layout.** Box geometry is asserted in the crate's host tests
  (`extensions/pdf/tests/`). A visual result is checked by rasterising the page with the image
  component's PDF decoding from Stage 3 and reading the PNG; a golden PNG is compared with
  `Image::compare` at a tolerance. A golden is never rewritten to make a check pass.
- **Determinism is part of every stage**: two renders of equal input are byte-equal, including
  encrypted output.
- **The tradeoffs**, stated here, and in the fragments and the commit, because AGENTS.md asks.
  Performance: one call per render, two with a header function; milliseconds for an invoice and
  seconds for a long report.
  Memory: nothing until first use; then the compiled module, shared by every core, and each rendering
  request's layout tree and output inside its own cap, freed when the request ends. The binary grows
  by the measured size of Stage 14. Usability: invoices, reports, labels and letters with no
  installation, headers and tables that survive page breaks, and e-invoices. Simplicity: one input
  language, one engine, no builder, and a CSS subset that is written down and tested.
- **Neutral names only** in every test, example and fragment: `Shop`, `Blog`, `example.com`.
- **Every comment in a new `.nvs` and every `about.md` this goal writes follows `AGENTS.md`
  § *Text an end user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over
  them before the wrap.
- **A debug cargo command never takes `-p`.** Narrow what runs with `bun nv verify -p nvs-ext` or a
  `--test` filter.

## Stages

### 2. the fragments, `wit/pdf.wit` and the font licence

**Does:** Writes the `Novis\Pdf` fragments § *Standing decisions* creates and rewrites, the WIT
interface the component exports, and admits the default fonts' licence.

One file set: `docs/rules/core-classes/`, `docs/rules/packaging/`, `wit/pdf.wit` (new),
`wit/image.wit`, `tools/nv/cmd/gen-attribution.ts:66`, `deny.toml:38`.

- **The fragments**, written first. Every class, member and options shape under `Novis\Pdf`, the
  new `Novis\Image` members (barcodes, SVG output, `open`'s `page` and `dpi`), the documented CSS
  subset as a table, the `Page` value and its marks, the table rules, the standards, the reader and
  `merge`, encryption, the error kinds, and the user's calls as decided, each with its reasons, a
  `**Memory:**` bullet where it spends memory, and a `**Declined:**` list, and the tradeoffs. The
  headless-browser backend, form filling, signatures, PDF417 and Aztec go on a `**Declined:**`
  list. It rewrites the seven `pdf-*` fragments under `docs/rules/core-classes/` to the new
  decisions and `rule:packaging/the-first-party-components-are-built-in`'s count, and creates at
  least these, under the ids the Stage 2 check reads: `core-classes/pdf-css-subset-table` (the
  subset as data a test reads), `core-classes/pdf-tables-across-pages`,
  `core-classes/pdf-header-is-a-page-function`, `core-classes/pdf-factur-x-is-the-one-attachment`,
  `core-classes/pdf-reader-and-merge`, `core-classes/pdf-default-fonts`,
  `core-classes/image-barcode-roster`.
- **`wit/pdf.wit`**, batch-shaped like `wit/intl.wit`: `render` (HTML, the asset map, options, and
  the per-page header and footer markup when a second pass carries it), `merge`, `read`, and the
  types they share. The two-pass header design (§ *Standing decisions*) is in it from the first write.
- **The proofs checks the fragments make possible**: one `bun nv proofs --verify --group '<class>'`
  check per class they name, in both components, added to Stage 16 of `data/goals/pdf.json`. They
  are written here and not with the goal, because a `--group` check behind a goal whose list runs
  the whole-roster gate holds that goal (`gatesInFrontOfGroups` in `tools/nv/lib/chain.ts`).
- **The font licence.** OFL-1.1 is admitted for **font data only**: `PREFERENCE` in
  `gen-attribution.ts` and the attribution the binary ships carry the Liberation fonts' licence and
  copyright. `deny.toml` gains it only if a crate in a component's graph declares it.
- **A `cargo deny` over each component's own graph.** None exists today: `deny.toml`'s `[graph]
  targets` omit `wasm32-wasip2` and `extensions/` is outside the workspace. The slice that adds it
  adds its check to this goal's checks.
- **Pinned by** the Stage 2 check.

### 3. SVG and PDF open as images

**Does:** `Novis\Image` decodes SVG through `resvg` and rasterises a PDF page through `hayro`, with
`page` and `dpi` passed from `Image::open` to the guest.

One file set: `extensions/image/src/decode.rs:60` (`sniff`), `extensions/image/src/info.rs:35`,
`extensions/image/src/lib.rs:130` (`Input::Encoded`) and `:446` (the WIT mapping that drops `page` and
`dpi`), `extensions/image/nvs/Image.nvs:61` (`open`), `extensions/fonts/` (new: Liberation, shared with
Stage 5), `crates/nvs-ext/tests/image_decode.rs`.

- **The two prerequisites first** (`rule:core-classes/pdf-page-is-an-image-source`): `hayro` builds for `wasm32-wasip2` and its graph
  passes the Stage 2 deny check. If either fails, the session records it and stops the PDF half with
  `BLOCKED`. There is no pdfium fallback (§ *Standing decisions*).
- **SVG** decodes with a `usvg` image resolver that only reads the asset bytes it is given; the
  default resolver reads files from disk and is never used. An external `href` throws.
- **PDF** follows `rule:core-classes/pdf-page-is-an-image-source` and
  `rule:core-classes/pdf-decode-refusals`: one page per `open`, priced by the pixel cap before a
  buffer exists, `info` counts pages without a pixel, a page past the end throws naming the count,
  an encrypted file throws `ParseError`, and the Liberation fonts stand in for the 14 standard fonts.
- **The bomb corpus** of `rule:core-classes/pdf-decode-refusals`: recursive XObjects, an xref loop, a stream-length
  lie and a deflate bomb each end in a throw or a contained trap.
- **Pinned by** the Stage 3 checks.

### 4. shaping and barcodes in `Novis\Image`

**Does:** `Image::text` shapes with `harfrust`, and `Novis\Image` renders 1D barcodes, DataMatrix,
and QR codes as SVG.

One file set: `extensions/image/src/text.rs`, `extensions/image/src/qr.rs`,
`extensions/image/src/barcode.rs` (new), `extensions/image/nvs/QrCode.nvs`,
`extensions/image/nvs/Barcode.nvs` (new), `crates/nvs-ext/tests/image_analysis.rs`,
`crates/nvs-ext/tests/image_barcode.rs` (new).

- **Shaping** replaces the first wave's Latin-only kerning (`text.rs:1`) with `harfrust`, with
  `unicode-bidi` for direction, so Arabic, Hebrew and Devanagari draw correctly.
  `rule:core-classes/image-one-entry-point-per-job` and M17
  Part 1 placed it here.
- **Barcodes** (`core-classes/image-barcode-roster`): Code 128 and GS1-128, EAN-13, EAN-8,
  UPC-A, Code 39 and ITF-14 through `barcoders`, DataMatrix through `datamatrix`. Each renders as an
  SVG string or as any raster format the component encodes. A wrong check digit or a character the
  symbology cannot carry throws `LogicError` naming it.
- **QR codes gain SVG output**, and an EPC payload helper (GiroCode) builds the SEPA payment text in
  Novis and passes it to the QR code.
- **The tests read every code back**: QR with `rqrr`, as today; 1D and DataMatrix with a reader that
  is a dev-dependency of the test only.
- **Pinned by** the Stage 4 checks.

### 5. the PDF component is built in

**Does:** Builds `extensions/pdf` into every binary as the third component, and renders a paragraph of
HTML to a byte-equal PDF with the default fonts and no grant.

One file set: `extensions/pdf/` (new), `crates/nvs-ext/build.rs:115` (`COMPONENTS`),
`crates/nvs-ext/src/builtin.rs:18`, `crates/nvs-ext/src/load.rs:667`, `crates/nvs-ext/tests/pdf.rs`
(new), `docs/rules/packaging/the-first-party-components-are-built-in.md`.

- **The crate** in `extensions/image`'s shape: its own `[workspace]`, `cdylib` and `rlib`, release
  with LTO and `panic = "abort"`. It points `html5ever` at `vendor/html5ever` (`Cargo.toml:1151`), so
  the PDF component parses with the same patched copy and the same 512-element cap as `Core\Html`.
- **The asset map** (`rule:core-classes/pdf-render-has-no-io`): a name resolves against the map and
  nothing else; `data:` URIs decode in place; `file:`, `http(s):`, an absolute path or a name the map
  lacks throws naming it. Nothing in the guest can fetch.
- **The writer** is `krilla` over `pdf-writer`. No timestamp unless the program passes one, a fixed
  document id derived from the content, and no hash-map order or float formatting that varies between
  runs. A scanner over the output finds no `/JavaScript`, `/Launch` or `/OpenAction`.
- **The fonts** are `extensions/fonts/`'s Liberation set; `sans-serif`, `serif`, `monospace`, Arial,
  Helvetica, Times, Times New Roman and Courier map to them.
- **Pinned by** the Stage 5 checks.

### 6. the cascade

**Does:** Computes every element's style from `<style>` blocks and `style=""` attributes, and reports
every declaration it dropped.

One file set: `extensions/pdf/src/css/` (new), `extensions/pdf/tests/css.rs` (new).

- **Our own cascade** over `cssparser` and `selectors`: the selectors the subset lists, specificity,
  inheritance, `!important`, `var()`, `calc()`, `@media print` and `all` applied and `screen` not.
- **The subset is data**: `extensions/pdf/subset.json` (new), which
  `core-classes/pdf-css-subset-table` points to and nothing restates. It lists every property,
  value function, at-rule, selector and PDF-specific HTML attribute (`data-repeat`, `data-mark`,
  `<slot>`) with a one-line plain description. A test reads it: every property it lists parses, and
  every property it does not list is dropped and reported with its selector and source position
  (`rule:core-classes/pdf-css-subset`). Stage 15 generates the editor's data from the same file.
- **Pinned by** the Stage 6 checks.

### 7. text, blocks, lists and images

**Does:** Lays out block and inline content with `parley`, with every script, the font chain,
hyphenation, lists, images and inline SVG.

One file set: `extensions/pdf/src/layout/` (new: block, inline, text), `extensions/pdf/tests/layout.rs`
(new).

- **Text** through `parley` with `harfrust`, `skrifa` and ICU4X, its font discovery off: alignment
  including justify, `line-height`, indent, decoration, transform, spacing, `white-space`,
  `word-break`, `overflow-wrap`, `text-overflow: ellipsis`, `vertical-align`,
  `font-feature-settings`, bidi, CJK line breaking, and `hyphens: auto` through `hypher`.
- **The font chain**: `@font-face` from the asset map, then the defaults. A character no font has is
  drawn as the font's empty box and listed in `missingGlyphs` with the chain it tried; it never throws.
- **Boxes**: margin, padding, borders (solid, dashed, dotted, double), `border-radius`, sizes with
  min and max, `box-sizing`, background colours, images and gradients, `outline`, `opacity`,
  `inline-block`, `display: none`.
- **Lists and counters**: markers (decimal, roman, alpha, disc), `counter-reset`,
  `counter-increment`, `::before` and `::after` with `content`.
- **Images**: JPEG, PNG, GIF and WebP, `object-fit`, inline `<svg>` and an SVG `<img>` as vector
  through `krilla-svg`. A raster image is priced by the request's memory cap.
- **Pinned by** the Stage 7 checks.

### 8. flex, grid, floats and positioning

**Does:** Adds flex and grid through `taffy`, floats and `clear`, `relative`, `absolute` and `fixed`
positioning with `z-index`, and 2D transforms.

One file set: `extensions/pdf/src/layout/` (flex, grid, float, position), `extensions/pdf/tests/layout.rs`.

- **Flex and grid** break across pages only between items and rows; a single flex line or grid row is
  never split.
- **`position: fixed`** repeats the element on every page, which is how a watermark is written.
- **Pinned by** the Stage 8 checks.

### 9. pagination

**Does:** Breaks the laid-out document into pages under `@page`, with margin boxes, running elements,
counters, page breaks, links, bookmarks and metadata.

One file set: `extensions/pdf/src/paginate/` (new), `extensions/pdf/tests/paginate.rs` (new).

- **`@page`**: size (named sizes, landscape, custom), margins, `:first`, `:left`, `:right`, `:blank`,
  named pages through `page`.
- **The 16 margin boxes**, `counter(page)` and `counter(pages)`, `string-set` and `string()`, running
  elements through `position: running()` and `element()`.
- **Breaks**: `break-before`, `break-after`, `break-inside: avoid`, the `page-break-*` aliases,
  `orphans` and `widows`.
- **A table of contents**: `target-counter()` and `leader()`, laid out again until the page numbers
  stop changing, which `counter(pages)` needs anyway. A document whose numbers never settle throws
  after a fixed number of passes the fragments name.
- **Links and structure**: `<a href>` to `http(s):`, `mailto:` and `#anchor`; bookmarks from `h1` to
  `h6`; `<title>`, `<meta>` and `lang` become metadata.
- **Pinned by** the Stage 9 checks.

### 10. header and footer functions

**Does:** `Document::header`, `::footer` and `::slots` take a Novis function that is called once per
page with a `Page` value, and a second call places what it returns.

One file set: `extensions/pdf/nvs/` (new: `Document.nvs`, `Page.nvs`, `Rendered.nvs`),
`extensions/pdf/src/paginate/`, `crates/nvs-ext/tests/pdf.rs`.

- **`core-classes/pdf-header-is-a-page-function`**: the component lays out the body once and
  returns each page's facts; Novis calls the function once per page with no crossing; one more call
  places the header and footer markup in the page margins. The instance keeps the laid-out body
  between the two calls (`rule:packaging/a-fresh-instance-per-request`, `crates/nvs-ext/src/call.rs:1`).
- **`Page`** carries the number, the count, first and last, the named page, the `string-set` values
  and the marks: `data-mark="amount:12.50"` on any element, several separated by `;`, read with
  `marks(name)` for that page and `marksUpTo(name)` for every page up to it, as strings for Novis to
  add up exactly. One fixed attribute name, not `data-mark-<name>`, because the editor's custom data
  completes fixed names only (Stage 15).
- **The margin is fixed** by `@page margin` or the function's `height`, so a header never moves the
  body. Markup taller than its area is listed in `overflows`. Where a margin box and a function both
  fill one margin, the function's markup is drawn.
- **Pinned by** the Stage 10 checks.

### 11. tables across pages

**Does:** A table keeps its columns, repeats its header and footer, never cuts a row through a line,
closes its borders at every break and fills its running values.

One file set: `extensions/pdf/src/layout/table.rs` (new), `extensions/pdf/src/paginate/`,
`extensions/pdf/tests/tables.rs` (new).

The user's eight rules (`core-classes/pdf-tables-across-pages`), each a default with no
configuration:

1. **Column widths are computed once for the whole table**, so every page has the same columns.
2. **`thead` repeats at the top and `tfoot` at the bottom of every page the table is on.** A row
   inside either says when it shows with `data-repeat`: `every` (the default), `first`, `last`, or
   `continued` (only where the table goes on).
3. **A row that fits on a page is never split.** A row taller than a page splits between lines of
   text, never through a line, an image or a glyph, and each cell continues on the next page.
4. **Borders are closed at every break**: the table's outer border and collapsed cell borders are
   drawn on both sides. `box-decoration-break: slice` on the table restores CSS's open edge.
5. **A header is never alone.** `orphans` and `widows` on a table count rows, default 2, and a table
   never starts at a page bottom with only its `thead`.
6. **A `<tbody>` and a `rowspan` group stay together** when they fit on a page, and split between
   rows when they do not.
7. **The caption shows on the first page only**, and `:nth-child` striping follows source order.
8. **`<slot name="…">` in a repeated row** is filled by `Document::slots(fn(Page $p) => [...])`.
   The slot keeps its row's height, so filling it never moves anything; text that does not fit is
   listed in `overflows`.

- **Pinned by** the Stage 11 checks.

### 12. standards, Factur-X and passwords

**Does:** Output is always tagged; `standard` selects PDF/A-2b, PDF/A-3b or PDF/UA-1; `invoiceXml`
embeds one Factur-X XML; `password` encrypts with AES-256.

One file set: `extensions/pdf/src/write.rs`, `extensions/pdf/src/encrypt.rs` (new),
`extensions/pdf/nvs/Document.nvs`, `crates/nvs-ext/tests/pdf_standards.rs` (new).

- **Tagging** from the HTML: headings, paragraphs, lists, tables with header cells, figures with
  `alt`, and the document language.
- **A document that breaks its chosen standard throws** naming what broke, through `krilla`'s
  validation: an image with no `alt` under PDF/UA-1, a missing font embedding under PDF/A.
- **Factur-X** (`core-classes/pdf-factur-x-is-the-one-attachment`): exactly one XML file,
  checked well-formed and inside the memory cap, under PDF/A-3b only, with the fixed file name, the
  `AFRelationship` and the XMP fields the standard requires. Any other attachment, and this one
  outside PDF/A-3b, throws. The XML is the program's to build.
- **A password** encrypts with AES-256 (revision 6) and nothing weaker. Its salts and file key are
  derived from a hash of the document and the password, so equal input stays byte-equal. A password
  with a PDF/A standard throws, because PDF/A forbids encryption.
- **Pinned by** the Stage 12 checks.

### 13. existing PDFs: placing, merging and reading

**Does:** A page of an existing PDF is placed like an image or as a page background, `Pages::merge`
combines and reorders pages, and `Reader` returns a PDF's page count, metadata, attachments and text.

One file set: `extensions/pdf/src/read.rs` (new), `extensions/pdf/src/merge.rs` (new),
`extensions/pdf/nvs/` (`Pages.nvs`, `Reader.nvs`), `crates/nvs-ext/tests/pdf_read.rs` (new).

- **Placing** uses `krilla`'s `pdf` feature over `hayro`: `<img src="terms.pdf#page=2">` and
  `@page { background-image: url(letterhead.pdf) }` draw the page as vector content. Only the page's
  appearance is kept; its links, form fields and bookmarks are not.
- **`merge`** takes PDFs with optional page lists and returns one PDF: merging, splitting,
  reordering and removing pages are that one call.
- **`Reader`** returns the page count, the metadata, the attachments (the Factur-X XML of a received
  invoice) and the text of a page in the order the file stores it, through a text output over
  `hayro`'s interpreter and the fonts' Unicode maps. It does not rebuild tables or columns.
- **An encrypted source throws** in all three, as on the image side.
- **Hostile sources stay inert**: the fixtures `rule:core-classes/pdf-decode-refusals` owes (a PDF
  with an external reference, one with a JavaScript action, one with an embedded file, and one per
  object the decoder cannot read) open through `Image::open`, placing, `merge` and `Reader`, and none
  of them reaches the disk, the network or a script.
- **SVG `<text>` is drawn**, in the PDF and in `Novis\Image`: `usvg`'s font database is filled with
  the Liberation faces (`extensions/pdf/src/layout.rs`, `rule:core-classes/image-format-roster`).
- **Pinned by** the Stage 13 checks.

### 14. the growth, the benches and a bundle

**Does:** Measures what the PDF component and the image second wave add to the binary, commits the
render benchmarks, and proves a bundle calls all three components.

One file set: `extensions/pdf/src/lib.rs` (its module doc), `crates/nvs-ext/tests/builtin.rs`,
`crates/nvs-cli/tests/bundle.rs`, `benches/members/novis/`.

- **The growth** is recorded in the component's module doc and held there by
  `the_recorded_growth_matches_the_embedded_components`. The goal writer's estimate was 6 to 9 MB, not
  checked.
- **Two render benchmarks**: one invoice and one 200-page report with a long table, guest time
  recorded beside the image component's figures. The text-page and image-page rasterising
  benchmark `rule:core-classes/pdf-page-is-an-image-source` owes goes beside them.
- **If the growth looks too large**, the session records the figure and puts the question in the
  handoff's `## Backlog`. Nothing becomes optional: that is the user's call.
- **Pinned by** the Stage 14 checks.

### 15. completion and hover in the editor

**Does:** VS Code completes and explains the PDF-specific HTML attributes and CSS inside every HTML
region and ``html`…` `` template, from data generated out of `extensions/pdf/subset.json`.

One file set: `editors/vscode/package.json:29` (`contributes`), `editors/vscode/data/` (new),
`editors/vscode/test/contributions/contributions.test.ts`, a generator under `tools/nv/` (new).

- **The editor's own HTML and CSS services already answer inside markup**
  (`rule:ide/a-template-region-gets-the-editors-services-and-formatter`), and the Novis server
  answers nothing there (`crates/nvs-lsp/src/completion.rs:1267`). So the work is custom data, not
  server code: `contributes.html.customData` and `contributes.css.customData` point at two generated
  files.
- **HTML**: `data-repeat` with its four values, `data-mark`, and `<slot>`'s use in a repeated row,
  each with a hover. **CSS**: what VS Code's own data lacks, among them `string-set`, `running()`,
  `element()`, `target-counter()`, `leader()` and the 16 margin-box at-rules, so they complete and the
  editor stops marking them unknown.
- **One home**: the generator writes both files from `subset.json`, and a headless test fails when
  they differ from what it would write. The hovers are text an end user reads (AGENTS.md § *Text an
  end user reads*) and link the reference page Stage 16 writes.
- **They show in every HTML region**, because the editor cannot know which template becomes a PDF.
  The attributes do nothing in a browser.
- **Pinned by** the Stage 15 check.

### 16. the feature proofs

**Does:** Writes the feature proofs for every member the fragments named, in both components.

One file set: `docs/examples/novis/`, `tests/hostile/novis/`, `benches/members/novis/`,
`docs/reference/novis/` (new pages for `Novis\Pdf`), `data/proofs/policy.json`.

- **What each member owes** (`rule:testing/feature-proofs`): `about.md`, tests from Novis and Rust,
  three examples, one bench, one attack and its help in the binary. `bun nv proofs --id '<member>'`
  prints what is owed.
- **The manifest class** follows `Novis\Image\Codec`'s rule: tests, attack and help in full;
  examples and bench in `data/proofs/policy.json`'s `skip`, one reason each.
- **The attacks**: `<img src="file:///etc/passwd">` and `http://169.254.169.254/` in HTML, CSS and
  SVG; HTML nested past the element cap; a table of 100,000 rows under a small memory cap; a
  `target-counter` loop that never settles; a font file that lies about its tables; a PDF bomb passed
  to `merge` and `Reader`; a Factur-X XML of a billion laughs.
- **The examples** are the cases users meet: an invoice with a carried-forward total, a long report
  with a table of contents, a shipping label with a barcode, a certificate in landscape, a contract on
  a letterhead PDF with terms appended, and an uploaded invoice's XML read back.
- **Pinned by** the Stage 15 checks.

## Handoff

### State

**Stage 14 is done: the growth, the render benches, the bundle and the rasterising bench are on disk, and both Stage 14 checks pass.**

- The rasterising bench is two more jobs of `crates/nvs-ext/benches/image_guest.rs` (`pdf_text_page`, `pdf_image_page`), built in `crates/nvs-ext/tests/support/image_arms.rs`, and its figures are in `benches/results/image-guest.json` and `rule:core-classes/pdf-page-is-an-image-source`. With only the size crossing back, guest against native is 13 against 9 ms for the text page and 53 against 31 ms for the image page. The raw RGBA8 of an A4 page at 150 dpi adds about 390 ms crossing back, one `Val` per byte.
- `crates/nvs-ext/tests/image_guest.rs` holds the image page byte-equal across the arms and the text page within one level, because glyph anti-aliasing rounds differently under the host's SIMD.

### Next

**Stage 15, completion and hover in the editor.** The file set is `editors/vscode/package.json`, a new `editors/vscode/data/`, `editors/vscode/test/contributions/` and a new generator under `tools/nv/cmd/`.

- [ ] **The generator** (`rule:core-classes/pdf-css-subset`): reads `extensions/pdf/subset.json:219` (its `html` list; `:2`, `:153`, `:171` and `:193` are the CSS lists) and writes VS Code's HTML and CSS custom data under `editors/vscode/data/`, with a `--check` that fails when they are stale.
- [ ] **The contribution**: `contributes.html.customData` and `contributes.css.customData` at `editors/vscode/package.json:29`, pointing at the two files; `data-repeat` with its values (the subset lists three where the stage's text says four), `data-mark`, and the CSS VS Code's own data lacks, as the stage's text lists.
- [ ] **The check**: a `describe` block beside `editors/vscode/test/contributions/contributions.test.ts:438` that the data is contributed, current and completes `data-repeat`'s values, so `npm --prefix editors/vscode run test:headless` holds it. Then Stage 16, the feature proofs.

### Backlog

- The PDF component's growth is 14.3 MiB, above the goal writer's 6 to 9 MB, and its fonts are a second 4.3 MiB copy beside the image component's: whether that stands is the user's call (`extensions/pdf/src/lib.rs`).
- Tables: a cell of a split row is not stretched to the row's height, a `rowspan` piece taller than a page is placed whole, a table without a `width` fills its containing block, a cell's `vertical-align` is `top`, a row's and a group's own background and borders are not drawn, collapsed borders are not resolved by style or width, a `data-mark` or `string-set` inside a cell counts as on the first page, and a `display: table` element directly in a row is a cell whose rows are block boxes (`extensions/pdf/src/layout/table.rs`, `extensions/pdf/src/layout/inline.rs`).
- Pagination: a `:left` or `:right` page wider or narrower than the first page of its name keeps that page's line widths, a margin box's counter other than `page` and `pages` is 0, a running element or `string-set` inside an inline-block, float or flex item counts as on the first page, and `element()`'s `start` and `first-except` read as `first` (`extensions/pdf/src/layout/margin.rs`). An `id` inside an inline-block, float, flex item, table cell or running element lands on no page, and `target-counter()` reads only `page` (`extensions/pdf/src/layout/inline.rs`). `break-inside: avoid` keeps only block boxes, and `break-after: avoid` and `break-before: avoid` do nothing (`extensions/pdf/src/layout/text.rs`). A link inside a transformed box keeps its box before the transform, and a heading inside an inline-block, float, flex item or table cell has no bookmark (`extensions/pdf/src/write.rs`).
- Positioning: an absolute box without a `width` is not stretched by `left` with `right`, a percentage `top` or `bottom` of a relative box is 0, an inline element's `position` and `transform` do nothing, and a transformed box is not a containing block (`extensions/pdf/src/layout.rs`, `extensions/pdf/src/layout/block.rs`). Floats: an `<img>` with `float` stays a block, a float in the middle of a paragraph is placed at its top, and a float does not go on to the next page (`extensions/pdf/src/layout/block.rs`). Grid: `calc()` tracks and the `grid-template` and `grid` shorthands (`extensions/pdf/src/layout/grid.rs`). Flex: `::before`, `::after` and `<br>` are not items, baseline alignment and an item's `margin: auto` (`extensions/pdf/src/layout/flex.rs`).
- Elliptical radii and repeating gradients (`extensions/pdf/src/layout/boxes.rs`), `hyphenate-character` (`extensions/pdf/subset.json`), an inline box in a right-to-left line, an SVG `font-family` of Arial, Helvetica or Courier, which falls back to Liberation Serif instead of mapping as HTML's does (`extensions/pdf/src/fonts.rs`), and Clippy over the component: `large_enum_variant` on `layout::Block` and `inline::Step`, and five small warnings in `extensions/pdf/src/css/value.rs:113` and `extensions/pdf/src/css/mod.rs:294` (`cargo clippy --manifest-path extensions/pdf/Cargo.toml --all-targets`).
- Header functions: the markup is styled by its own `<style>`, it sits at the top of its area, and `Page::string` has no conformance case (`extensions/pdf/src/lib.rs`, `extensions/pdf/nvs/Page.nvs`). `Document.nvs`'s `invoiceXml` doc names `Standard::PdfA3b` where a program writes `Document::Standard::PdfA3b` (`extensions/pdf/nvs/Document.nvs:52`). The password's user path and `/Perms` checked by a second reader, `hayro`'s `embed-cmaps`, and veraPDF in CI (the user's call) (`extensions/pdf/src/encrypt.rs`).
