# Why `tainted` and `secret`

**Injection and secret leakage are compile errors in Novis, not findings from a scanner run afterwards.**
That is the first of the three claims `rule:programs/three-claims` makes on the
project's behalf. This page is that claim written for the three people who ask about it, plus the comparison
that backs it. The mechanics live in the ADRs at the foot; nothing here restates them.

## The claim, for three readers

**Curious, not technical.** Novis can tell the difference between text a developer wrote and text a stranger
typed into a website, and it refuses to build software that confuses the two. Mixing those up is one of the
most common ways websites get broken into, and one of the most common ways passwords end up somewhere they
shouldn't be.

**Security and compliance.** Data that came from a user and data that must stay secret — passwords, keys,
tokens — are tracked by the language itself, so code that would put user data into a database query or a web
page, or write a credential into a log file, cannot be built or shipped at all. That makes injection and
credential leakage a *preventive* control rather than a detective one: no scan to run, no findings to
triage, and no way for a developer to skip it.

**Developers.** Never call `htmlspecialchars()` again, never scrub a credential out of a log, and never
argue about escaping in code review. The safe path is the short one — `echo` escapes for you, bound
parameters take user input as-is, and the compiler only speaks up when you are genuinely doing something
dangerous. You will write `tainted` in a few signatures, and once in a while you will tell the compiler you
know better: in one call, with a written reason, that a reviewer can grep for.

## What the other web languages have

| | Untrusted input | Secrets | Who enforces it |
|---|---|---|---|
| **PHP** | Nothing in core. `htmlspecialchars()` if you remember it; the `taint` extension was never core and is abandoned | Nothing | You, every time |
| **JS / TypeScript** | Nothing in the language. Trusted Types guards browser DOM sinks only — client-side, opt-in through CSP, silent about SQL and shell | Nothing | An optional header, in the browser |
| **Python** | Nothing in core. Jinja2 and Django auto-escape HTML — one library, one sink | Nothing | Your framework choice |
| **Java** | `@Tainted`/`@Untainted` exist, as Checker Framework annotations. You annotate third-party libraries yourself | Nothing | A tool you can skip |
| **C#** | Roslyn analyzers, CodeQL. They produce *findings* to triage, not errors | Nothing | A scan someone suppresses |
| **Go** | `html/template` auto-escapes contextually and does it well — but `text/template` sits beside it with the same API and no escaping | Nothing | Which import you typed |
| **Ruby** | Had `taint`/`untaint` and `$SAFE`. Deprecated in 2.7, **removed in 3.0** | Nothing | Nobody, any more |
| **Rust** | Nothing in core. Typed-HTML crates, `secrecy` | `secrecy`, opt-in | Which crate you picked |
| **Novis** | `tainted` — a type, applied automatically to every input, refused at every sink | `secret` — a type, refused at logs, screens, traces and serialization | The compiler |

Research and crypto languages have gone further than any of the above — Jif and FlowCaml carry full
information-flow labels, Ur/Web makes an injection unrepresentable by typing SQL and HTML constructively,
and FaCT and CT-Wasm have a literal `secret` qualifier for constant-time code. None of them is a
general-purpose language anyone ships a web application in, and Ur/Web's constructive SQL is the one design
whose guarantee is genuinely stronger than the rule
[ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md) § *Revisiting* already flags for revisiting.

## Why this is a difference in kind

1. **It attaches itself.** Every other system on that list needs a human to mark what is untrusted. Novis
   has five named entry points for outside data (`rule:statements/no-host-populated-variables`), so it marks them
   for you — and the same standing rule covers data read back out of a store, which is what closes stored
   injection by the same mechanism as reflected.
2. **A sink is a predicate, not a list.** Everyone else enumerates dangerous functions, so every new API is
   a new hole. [ADR 0088](adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 2 makes an
   unclassified parameter refuse a tainted argument, so a member nobody classified fails closed.
3. **No generic `sanitize()`.** Perl and Ruby both shipped taint and both let you launder with a regex,
   which is why both became theatre. Novis has one launderer per sink, named for that sink, or none
   ([ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md) § 3).
4. **Not skippable, and no annotation debt.** It is `nvs check`, not a pass beside it — nothing to
   configure, nothing to suppress, and no third-party library to annotate first. That last cost is why Java
   has the annotations and almost nobody uses them.
5. **`secret` exists at all.** Outside cryptography DSLs, no general-purpose language has it
   ([ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)).

## What not to claim yet

**Not "XSS is impossible."** Auto-escaping closes text-node XSS; attribute context, `javascript:` URLs,
inline `<script>` and CSS context need the context-specific escapers that
[ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md) § 3 still owes as stdlib design, and
`Core\Html::sanitize` is the hardest item on that roster. Until they land, the sentence this project may
write is *injection is a compile error* — which is the stronger claim anyway, because it is about who
enforces it rather than about how complete the escaping is.

**Not "faster than PHP" alongside it.** `rule:programs/three-claims` retired
that as a headline, and pairing a retired claim with a live one weakens both.

## Where the detail is

| Topic | File |
|---|---|
| `tainted`: how it enters, propagates and is laundered; the sinks that refuse it; HTML auto-escape and `Core\Html\Markup` | [ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md) |
| `secret`: the second qualifier, its sinks, and why it has no ambient source | [ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md) |
| What makes something a sink, and why an unclassified one refuses | [ADR 0088](adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md) |
| Where untrusted data enters a program at all | `rule:statements/no-host-populated-variables` |
| A launderer's return type, and why only an idempotent escape answers a `string` | [ADR 0133](adr/0133-a-launderer-answers-its-sinks-carrier-and-only-an-idempotent-escape-answers-a-string.md) |
| Both qualifiers at an extension boundary | [ADR 0055](adr/0055-extension-qualifier-declarations.md) |
| A `secret` value concealed in the editor | [ADR 0101](adr/0101-secret-is-redacted-in-the-editor-and-the-range-comes-from-the-server.md) |
| Who Novis is for, and the three claims it makes about itself | `rule:programs/audience` |
| How a developer writes either qualifier | [the reference](novis.md), chapter A.2 |
