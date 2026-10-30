Methods, properties, class constants and enum cases keep the order their author wrote when `nvs fmt`
runs, because declaration order is observable: `rule:core-classes/derive-property-list` makes a derived
codec's encode order the property declaration order, on purpose, for ETags and cached fixtures; and
`rule:testing/bench-counters` makes the test runner's report order the declaration order of the cases.
A formatter that reordered members would change what a program prints and sends, and a formatter that
changes meaning is not a formatter — the same line `rule:core-api/written-visibility` takes when it
refuses to let the formatter insert a missing `public`.

The member order exists (`rule:tooling/class-members-have-one-order`), and a developer gets it only by
asking: the editor's *Order class members* source action, which no save runs, or `nvs code order`.
Both write a diff the developer reads, and both name a derived class whose JSON key order the move
changes. Inside `nvs fmt`, the `use` block (`rule:tooling/fmt-sorts-the-use-block`) is the only
reordering.

**Declined:**
- **`nvs fmt` writing the member order**, by default or behind a flag. `fmt` reads one file and runs
  on save, while the order needs names resolved across the project (`Core\Json\Derive`,
  `Core\Json\KeepKeyOrder`) and must never run on save. A flag on `fmt` would be a second name for
  `nvs code order`.
