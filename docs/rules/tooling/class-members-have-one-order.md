The body of a class and of an interface has one member order, the same in every project. Novis has
no anonymous class (`new class { … }` is `E0244` at the parse). Top to bottom:

1. **Group:** `enum` and `type` declarations, then constants, then properties (hooked ones included),
   then methods.
2. **Inside a group:** every `static` member, then every instance member.
3. **Inside the static members, and again inside the instance members:** `public`, then `protected`,
   then `private`. The read visibility counts, so `public private(set)` is `public`.
4. **Inside one visibility:** `abstract`, then `final`, then the rest.

Steps 2 to 4 read only the modifiers a member writes. An interface method writes no `abstract`, so it
sorts with the rest, and an `enum` or `type` declaration has none of these modifiers, so its group keeps
its written order.

**Equal members keep their written order**, because the sort is stable. That is what keeps a test
class's report order (`rule:testing/bench-counters`): every `#[Test]` method is `public`, so the cases
keep their order among themselves. **The constructor is the first instance method**, after every static
method, whatever its visibility and modifiers: a `private` constructor behind a `public static` factory
is common, and a reader looks for the constructor in one place.

**What does not move:** a promoted constructor parameter stays in the constructor, an enum's cases keep
their order because `cases()` returns them in it, and hooks stay inside their property. **What moves
with a member:** its `///` run, its attributes, every comment between it and the member above it (the
formatter's own reading, `rule:tooling/fmt-base-style-is-per`), and a comment at the end of its last
line. A comment after the last member stays last. A joined declaration (`public int $a, $b;`) moves as
one unit.

A derived codec's key order is its declaration order (`rule:core-classes/derive-property-list`), so the
order is a change to what a program sends. A class carrying `#[Core\Json\KeepKeyOrder]` keeps its
instance properties in written order
(`rule:core-classes/keep-key-order-freezes-a-derived-classs-keys`), and every other derived class whose
field order changes is named by `nvs code order` before anyone ships it.

**Two clients write the order, and both only on request.** The editor's *Order class members* source
action (`source.orderMembers.nvs`) edits the whole file, and is never answered under `quickfix` or
`source.fixAll.nvs`, so no save runs it. `nvs code order` writes, checks (`--check`) or previews
(`--dry-run`) the order over files or the whole project. Both call one function, which moves each
member's text and then formats the result, so what they write is a file `nvs fmt --check` accepts. A
file with a syntax error gets no edit. `nvs fmt` never reorders (`rule:tooling/fmt-never-reorders-members`).

**Declined:**
- **A setting for a project's own order.** One order for every project is the point: a reader who
  knows one Novis codebase knows where a member is in all of them.
- **Grouping a private helper under the method that calls it.** The order would depend on a call graph
  and change with every edit.
- **Sorting by name.** It moves members on every rename.
