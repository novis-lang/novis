A codec is generated only where a class asks for one, with one attribute per format: `#[Json\Derive]`
and `#[Db\Derive]` on the class, `#[Json\Field]` and `#[Db\Field]` on a property. The attribute makes
the class implement the corresponding interface; writing `implements Core\Json\Codec` beside it is
redundant but accepted.

**Opting in is answered at the call as well as at the declaration.** A member that builds an instance
out of a document or a row names the class in the call itself, so that is where the compiler asks
whether the class participates at all: one carrying neither the format's attribute nor the
hand-written half the derive would have generated — `fromJson` for a document, `fromRow` for a row —
is refused while compiling, `E0821` at a document door and `E0806` at a row's. Both doors answer the
same way, and both answer before the program runs, so a class no document could ever be read into is
not something the first request to reach the member discovers. The run-time refusal stays underneath
as the backstop for a descriptor built by hand, which no call site named.

**A compiler-recognized attribute is matched nominally.** The compiler acts on an attribute only when
its name *resolves*, through the ordinary namespace and `use` rules, to one of a closed `Core`-owned
list. So `#[Core\Json\Derive]` and a `use`d `#[Derive]` are one attribute reached two ways, while a
userland `type Derive = {};` is not it no matter how it is spelled, and a bare `#[{...}]` payload object
never triggers one because it resolves to no name at all. An import binds a whole short name and is
never a namespace prefix, so `use Core\Json;` followed by `#[Json\Derive]` names nothing.

This is a carve-out of exactly one sentence, and it is the only one: attribute *retrieval* stays
structural, unchanged (`rule:attributes/structural-retrieval`). Every future compiler-recognized
attribute joins the closed list; nothing else is ever matched by name. The list itself is a single
table in the compiler (`nvs_types::derive::ATTRIBUTES`), and a rule adding an entry says its attribute
joins the list and states no running total, because a count restated beside each entry goes stale.

The nominal match is what lets the compiler generate code from an attribute at all: it acts on
**this** attribute, and never on a framework's own literal that happens to have the same shape. The
list is also the control on the attribute surface. Each entry is cheap to add, and a set of
attributes added because they were cheap is a second, undocumented language, so each entry is
argued on its own.

**Declined:**
- **Matching a compiler-recognized attribute structurally**, the way retrieval matches. The
  derive's payload is an empty shape, which structurally matches every attribute in a program, and
  even a non-empty one would generate code from a third-party literal that happens to fit.
- **One shared `#[Derive]` and `#[Field]` for both formats.** It forces a JSON key and a column
  name to be the same string, which is wrong often enough that two attributes would be needed at
  once. A class using both formats with the same rename writes it twice, and that is visible.
