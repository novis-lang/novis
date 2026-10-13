A header, a footer or a table slot that needs the program's own logic is a **Novis function called once
per page** with that page's `Page`, returning ``html`…` `` markup. Headers that need no logic are CSS:
the 16 margin boxes, `counter(page)`, `counter(pages)`, `string-set` and running elements, all in the
subset (`rule:core-classes/pdf-css-subset-table`).

**A render with a function is two calls to the component.** The first lays out the body, keeps the laid-out
pages in the request's instance (`rule:packaging/a-fresh-instance-per-request`) and returns each
page's facts with no file. Novis then calls each function once per page, with no crossing. The second
call carries every page's markup, places it in the margins and slots of the kept pages, and writes the
file. `wit/pdf.wit`'s `render` takes either pass, and a second pass with no first in the instance
returns `invalid`.

**A header never moves the body.** Each margin's height is fixed before the body is laid out, by
`@page`'s margin or the function's `height` option, so the page breaks the function saw are the page
breaks of the file. Markup taller than its margin is clipped to it and listed in `Rendered::overflows`
with its page: markup that goes on to a second page of its area, and a run of text or an image that
reaches below the area's bottom on its one page. A slot keeps its row's height, so filling it moves
nothing either, and its fill's `body` has no margin unless the fill's own `<style>` gives it one, so
the fill starts at its content box's top-left corner. Where a margin box and
a function both fill one margin, the function's markup is drawn.

**`Page`** carries the number, the count, whether it is the first or the last, its named page, its
`string-set` values and its **marks**. A mark is `data-mark="name:value"` on any element, several
separated by `;`. `marks(name)` returns the values on that page and `marksUpTo(name)` those on every
page up to it, as strings, so a carried-forward total is `Core\Decimal` arithmetic in Novis and never
rounds through a float.

- **Memory:** between the two calls, the request's instance keeps the laid-out body, inside the
  request's cap, and drops it with the instance. Novis holds one `Page` per page and the markup the
  functions return.

**On disk.** The CSS half is `extensions/pdf/src/layout/margin.rs`, and the two passes are
`first` and `second` in `extensions/pdf/src/lib.rs`, called by `Document::render` in
`extensions/pdf/nvs/Document.nvs`. A function's `height` is an `@page` rule after the document's
own, so a `:first`, `:left`, `:right`, `:blank` or named-page rule's margin on that side still wins
over it. `Document::slots` fills each `<slot name>` through the same second pass, in its cell's
content box.

**Declined:**
- **A function called from the guest per page.** The world has no export a guest calls back into
  Novis through, and a crossing per page costs more than one batch.
- **Laying the body out again with the headers in place.** A header that moved the body would change
  the page breaks the function saw, and the page count with them.
- **Running totals computed by the engine.** Which marks add up, in which currency and rounding, is
  the program's; the engine reports where each mark landed.
