- **A `-p nvs-stdlib` test cannot call another module's walk, because the guard that refuses a value
  is private to the module that owns it.** `csv::write_record`, `encoding::bytes_of` and
  `uri::scalar_text` all are, so a test asking several modules one question reaches their members
  the way a program does: `nvs_helper!` generates a `pub` symbol per member, and
  `nvs_runtime::call(crate::csv::nvs_core_csv_format, &mut Ctx::buffered(), &args)` drives one, with
  `ctx.take_pending()` for the refusal. Build the argument list its parameters take and release it
  yourself — the callee borrows its args. [until: reviewed 2026-09-09]
