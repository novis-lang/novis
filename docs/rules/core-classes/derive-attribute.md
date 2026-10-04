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
table in the compiler, which is what stops a per-record running total from going stale.
