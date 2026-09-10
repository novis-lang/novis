``html`…` `` is an expression of type `Core\Html\Markup` whose literal segments are trusted because the
author wrote them, and whose `{$…}` holes are escaped through `Core\Html::escape` and spliced.

```nvs
tainted string $name = $row->get("name");

echo html`<span>posted by </span>{$name}`;
Core\Html\Markup $badge = html`<span class="badge">new</span>`;
```

That is the whole rule, and it is the rule `<?= ?>` already follows
(`rule:core-classes/html-auto-escape`). A segment is always source and a hole is always escaped, so
nothing here turns computed text into trusted markup — the bypass `as Core\Html\Markup` closes by
admitting only a literal token stays closed by construction. A hole already holding a `Markup` is
spliced raw, which is `Markup + Markup` written in interpolation syntax.

**Backticks, and a named prefix.** HTML is full of `"`, so a double-quoted form would put a backslash in
front of every attribute; the backtick is free because Novis has no shell-execution form
(`rule:core-classes/process-is-argv-only`). A literal backtick in the body is `` \` ``. The prefix is a
name rather than a bare delimiter so that a second carrier, if one ever earns a literal, can say which
sink it means.

**A hole is `{$`; every other brace is text.** The hole grammar is a double-quoted string's
interpolation grammar, both halves of it and nothing added: `{$` opens a hole whose body is a **full
expression** closed by the matching `}`, with brace depth counted so a closure inside one does not close
it early — `{$u->fullName()}`, `{$row["name"]}` and `{$a + $b}` are all holes — and a bare `$name`
interpolates in PHP's simple syntax, `$name`, `$name->prop` one level, `$name[offset]`. **A hole must
begin with `$`**, so `{Money::format($c)}` is text exactly as it is in a double-quoted string, and a
static call reaches a hole through a local or a closure. Every other `{` is text, so a `<style>` block's
braces need no escape, and `\{` is the one case that wants a literal `{$`.

A `secret` value in a hole is refused where it is written (`rule:security/secret-sinks-refuse`); a
`tainted` one is accepted, because the sink never distinguishes the two. An unterminated literal is
`E0002`, the code that already covers an unterminated string, heredoc and interpolation — this rule adds
no diagnostic of its own.

**The compiler learns no HTML.** Segments are opaque bytes and only `{$` and the closing delimiter are
scanned for, so there is no tag tracking, no balance requirement and no rule about where a literal may
begin or end — ``html`<table>` `` and ``html`</table>` `` are both ordinary literals, which is what lets
a page be composed from fragments. It follows that a hole in a position element-text escaping does not
cover — an unquoted attribute, a URL-valued one, a `<script>` body — is accepted and produces exactly
what `<?= ?>` produces there. That is the sink's blind spot, identical in both spellings, and it is not
this rule's to close.

**What it costs to run.** A literal with no holes is a compile-time constant folded into the constant
pool, as a duration literal is (`rule:types/duration-literal`), so it allocates nothing per execution
where `as Core\Html\Markup` allocates one object per lift. A literal with holes in a sink position
lowers to a run of writes — segment, escaped hole, segment — with no carrier materialised, since a value
born and consumed at one sink is unobservable. In value position it is one `Markup` holding the joined
bytes.

`Core\Html::join(array<Core\Html\Markup> $parts, Core\Html\Markup $separator): Core\Html\Markup`
concatenates a list of fragments; every element is already a carrier, so it neither trusts nor escapes
anything.

**A carrier earns a literal form when its content is authored as text.** HTML markup is — `<span>` is
bytes a developer types. Terminal styling is not, by decision
(`rule:tooling/styling-is-a-value-not-a-grammar`), so `Core\Cli\Text` gets no matching form: its
segments could only ever be plain text, which needs no trust, because
`rule:tooling/terminal-output-is-a-sink` neutralizes every value regardless of qualifier and a bare
string is already accepted everywhere the carrier is. A third carrier is measured against that
predicate rather than against the count.
