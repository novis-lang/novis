`echo` inside an HTTP request is an escaping sink. It accepts only `Core\Html\Markup`, implies
`Content-Type: text/html`, and auto-escapes any non-`Markup` value interpolated into a
`Markup`-building position, lifting the result. It never distinguishes tainted from untainted,
because escaping neutralizes either one structurally. Every other body shape is a typed response
member — `json`, `text`, `bytes`, `sendFile` — each framing its own content, and mixing `echo` with
one of them on a single response is a compile error.

This is a deliberate exception to the standing rule that nothing happens by position, only by
declaration. Security ranks above simplicity, and an omitted escape call is the single most common
real-world XSS root cause, so the priority is spent explicitly rather than holding the no-magic line
for its own sake. It is one of only two such exceptions.

`Markup` is a small value type, peer to `string` the way `bytes` is. A **string literal**
converted with `as Markup` is trusted — it is exactly what the developer wrote. A runtime-computed or
`tainted` string can never become `Markup` that way, which closes the obvious bypass.
`Markup + Markup` is `Markup`, so composing trusted fragments stays cheap; `.` has no row for a
carrier, and a mixed `$markup + "x"` is refused rather than escaped, because `+` is not a sink.

**In expression position the ordinary spelling is an html template**, not the lift and the operator:
``html`<span>posted by </span>{$name}` `` is a `Markup` whose segments carry the same trust `as Markup`
grants a string literal and whose holes are escaped by this rule
(`rule:core-classes/html-template`), which is also where `Core\Html::join` composes a list of fragments.
`as Markup` and `+` keep their meaning and become the narrow forms — a string literal already held in an
initializer, and two computed carriers.

**A placeholder from `Core\Html::later` is a `Markup` the runtime makes**, whose bytes the developer
never writes (`rule:core-classes/html-later`). It carries a per-request token no author text and no
escaped visitor string can contain, so it composes with `+`, `Core\Html::join` and a partial like any
other fragment, and `Markup` keeps its one slot.
