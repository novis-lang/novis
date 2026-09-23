- **A `Core` class's registry row may carry a name that is a constant from another module, so grepping
  `registry.rs` for the literal class name finds nothing and the row looks absent.** `Core\Html\Markup`
  is `crate::html::MARKUP` in `registry::CLASSES` under `MARKUP_NAME`, which is
  `nvs_runtime::CARRIER_HTML_MARKUP` — a search for `Core\\Html\\Markup` in `registry.rs` returns zero
  hits, which nearly cost a filter over `CLASSES` the one class the `` html`…` `` constant needs. Grep
  the owning module for `pub const CLASS`/`NAME`, or `registry.rs` for `crate::<module>::`, and confirm
  membership there rather than by the class's spelled name. [until: reviewed 2026-12-01]
