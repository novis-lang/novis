An **html template**, ``html`…` ``, is an expression of type `Core\Html\Markup` whose text segments are
trusted because the author wrote them, and whose `{$…}` holes are escaped through `Core\Html::escape`
and spliced.

```nvs
tainted string $name = $row->get("name");

echo html`<span>posted by </span>{$name}`;
Core\Html\Markup $badge = html`<span class="badge">new</span>`;
```

That is the whole rule, and it is the rule `<?= ?>` already follows
(`rule:core-classes/html-auto-escape`). A segment is always source and a hole is always escaped, so
nothing here turns computed text into trusted markup — the bypass `as Core\Html\Markup` closes by
admitting only a string literal stays closed by construction. A hole already holding a `Markup` is
spliced raw, which is `Markup + Markup` written in interpolation syntax.

**Backticks, and a named prefix.** HTML is full of `"`, so a double-quoted form would put a backslash in
front of every attribute; the backtick is free because Novis has no shell-execution form
(`rule:core-classes/process-is-argv-only`). A backtick in the body is written `` \` ``. The prefix is a
name rather than a bare delimiter so that a second carrier, if one ever earns a template form, can say
which sink it means.

**Two holes: `{$…}` as in a string, and `<?= … ?>` as in a page.** The first is a double-quoted
string's interpolation grammar, both halves of it and nothing added: `{$` opens a hole whose body is a
**full expression** closed by the matching `}`, with brace depth counted so an anonymous function inside one does not
close it early — `{$u->fullName()}`, `{$row["name"]}` and `{$a + $b}` are all holes — and a bare `$name`
interpolates in the simple syntax, `$name`, `$name->prop` one level, `$name[offset]`. A brace hole
must begin with `$`, so `{Money::format($c)}` is text exactly as it is in a double-quoted string, and
every other `{` is text, so a `<style>` block's braces need no escape; `\{` writes a plain brace.
A brace directly before a class path — `{Page::TITLE}`, `{Money::format($c)}` — is the one text a
page prints that was almost never meant, so it is warned about where it is written, `W1012`, with the
help naming `<?= Page::TITLE ?>` and `\{`; the bytes stay text, and a `$name` inside them still
interpolates. It is a warning and not a refusal because the program is correct; only its page prints
the wrong string. A double-quoted string says nothing: its grammar is PHP's own, and a string is not
a page.
The second is the output tag a page already uses: `<?= expr ?>` opens a hole that takes **any**
expression and closes on the first `?>` outside a nested string or template, so a constant, a static
call and a nested template reach the page without a local — `<?= App::VERSION ?>`,
`<?= Money::format($c) ?>`, `<?= $on ? html`<b>on</b>` : html`<i>off</i>` ?>` — and a `}` inside it
is an ordinary brace. `<?=` occurs in no HTML, CSS or JavaScript, so no page text needs an escape
to make room for it. Both holes are escaped by the same rule;
the tag differs from the brace only in what it lets in. A double-quoted string takes no `<?=`: a
string is not a page, a `<?=` in one is text, and a code generator that writes PHP into a string
would break if it were a hole. A `<?nvs` tag inside a template is `E0010`, reported once at the tag
with `<?= expr ?>` and composition named, and is then text: a template is one expression, a code
block inside it would need a new mechanism to capture statements' output as a value, and a loop
around markup is code mode outside it.

A `secret` value in either hole is refused where it is written (`rule:security/secret-sinks-refuse`); a
`tainted` one is accepted, because the sink never distinguishes the two. An unterminated template or hole
is `E0002`, the code that already covers an unterminated string, heredoc and interpolation. `E0010` is
the one diagnostic this rule adds, for the code block a template cannot hold.

**The compiler learns no HTML.** Segments are opaque bytes and only `{$`, `<?=`, `<?nvs` and the closing
delimiter are scanned for, so there is no tag tracking, no balance requirement and no rule about where a
template may begin or end — ``html`<table>` `` and ``html`</table>` `` are both ordinary templates, which
is what lets a page be composed from fragments. It follows that a hole in a position element-text
escaping does not cover — an unquoted attribute, a URL-valued one, a `<script>` body — is accepted and
produces exactly what `<?= ?>` produces there. That is the sink's blind spot, identical in both
spellings, and it is not this rule's to close.

**What it costs to run.** A template with no holes is a compile-time constant folded into the constant
pool, as a duration is (`rule:types/duration`), so it allocates nothing per execution where
`as Core\Html\Markup` allocates one object per lift. A template with holes in a sink position lowers to
a run of writes — segment, escaped hole, segment — with no carrier materialised, since a value born and
consumed at one sink is unobservable. In value position it is one `Markup` holding the joined bytes.
A hole whose value is a constant is still escaped on every execution, as every hole is. Scanning
for `<?=` is one three-byte test beside the two-byte test for `{$`, in the template body only, and a
tag hole reaches the parser as the same part a brace hole does. `W1012` adds one byte test per `{`
in a template's text, at compile time only.

`Core\Html::join(array<Core\Html\Markup> $parts, Core\Html\Markup $separator): Core\Html\Markup`
concatenates a list of fragments; every element is already a carrier, so it neither trusts nor escapes
anything.

**A carrier earns a template form when its content is authored as text.** HTML markup is — `<span>` is
bytes a developer types. Terminal styling is not, by decision
(`rule:tooling/styling-is-a-value-not-a-grammar`), so `Core\Cli\Text` gets no matching form: its
segments could only ever be plain text, which needs no trust, because
`rule:tooling/terminal-output-is-a-sink` neutralizes every value regardless of qualifier and a bare
string is already accepted everywhere the carrier is. A third carrier is measured against that
predicate rather than against the count.

**What it spends is surface, not memory.** It is one more literal form, with a `nvs fmt` layout, an
editor region and a converter row behind it, and a reader meets two ways to write markup: a page of
inline HTML, and an expression that returns a fragment. It also makes the sink's blind spot easier to
write into: ``html`<a href="{$url}">` `` lets a `javascript:` URL through exactly as
`<a href="<?= $url ?>">` does, and convenience raises how often that is written. In return, PHP's
`'<b>' . htmlspecialchars($x) . '</b>'` has one faithful Novis form, ``html`<b>{$x}</b>` ``.

**Declined:**
- **A template that refuses the holes it cannot escape correctly**: an unquoted attribute, a URL
  attribute, an `on*` handler, a `<script>` or `<style>` body, a tag or attribute name, a comment body.
  It would make the template stricter than `<?= ?>`, so the same hazard would be an error in the newer
  syntax and legal in the older one. It would also need the compiler to hold an HTML position model.
  The hazard belongs to the sink, where one answer covers both spellings.
- **A bare string literal lifted implicitly in a sink position.** `echo '<b>' . $x` and
  `echo '<b>', $x` would produce different documents with nothing visible to say why, and `+` would
  stay as it was.
- **No template form, leaving fragments to the framework's template files.** `Web\Response::view`
  renders a page (`rule:programs/first-party-framework`); a function returning a table row or a
  component is expression position, which is where the ceremony was.
- **A ``text`…` `` form for `Core\Cli\Text`, for symmetry.** It fails the predicate above.
- **`{Name::…}` opening a brace hole.** It covers a constant and a static call and nothing else, so
  `{new App()}` and `{1 + 2}` stay text, and it adds a clause to the string grammar that a string
  does not have.
- **A second brace opener, `{= … }`, `{> … }` or `{{ … }}`.** Each collides with a template language
  a page may embed: `{{` is Vue, Twig and Handlebars, `{>` is a Handlebars partial and `{=` is
  Mustache's delimiter tag. Each would force `\{` escapes into client-side templates.
- **Refusing `{Class::…}` in a template.** It would grow the hole grammar by a negative, a brace that
  is text unless it looks like this, and the program it refuses is correct.
- **`W1012` inside a double-quoted string.** Every PHP programmer already knows that limit of the
  string grammar, and the defect the warning exists for is a wrong string reaching a page.
