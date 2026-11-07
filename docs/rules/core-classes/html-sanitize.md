`Core\Html::sanitize` answers `Core\Html\Markup`, which makes it the fourth way to obtain one and the
only one that takes a runtime-computed string.

That is exactly why it must be a parser that **rebuilds the document from a known-good grammar**, and
never a filter that deletes what looks dangerous. A filter answering a carrier would be a generic
sanitizer wearing a type — it would claim a guarantee it cannot establish, because "what looks
dangerous" is a list an attacker gets to extend.

**The rebuild is three steps and only the middle one holds a policy.** The document is parsed by the
WHATWG algorithm, which cannot fail, in no-quirks mode whatever doctype it has or lacks, because its
answer is written into a page with `<!DOCTYPE html>`. In quirks mode `<table>` does not close an open
`<p>`, so `<p>a<table></p>` would rebuild as a `p` inside a `p`, which no parse builds again.
`Core\Html::parse` keeps quirks mode for a document with no doctype, as a browser does for a page. A
new tree is built from a closed list of elements, each with the
attributes it may carry; and that tree is written back out under the serialization rules of the door it
came through. An element the list does not hold contributes no tag — dropped with its content when its
content model is raw text, since a `<script>`'s character data is neither markup nor prose, and
unwrapped otherwise, because `html`, `head` and `body` are elements the algorithm inserts around every
fragment. An attribute the list does not grant is not written, and a `href` or `src` naming a scheme
outside the ones that fetch is not written either.

**A document that fostered a node or lost a wrapper is rebuilt twice.** Foster parenting moves content
out of a table into the element around it, and so builds nesting no source builds:
`<ul><li>a<table><tr><li>b` puts one `li` inside another, and a second parse of the answer closes the
first. Unwrapping an element the list does not hold, such as an `applet` between two `li`, does the
same. So when the parse fostered a node, or the rebuild unwrapped an element other than `html`,
`head` and `body`, the answer is parsed and rebuilt once more, up to three passes in all. A document
with neither costs one pass. One with either costs a second parse of an answer no larger than itself,
and the extra latency is spent on the mutation-XSS property below.

**A relative reference never names a host.** A URL value that starts with two of `/` and `\`, once tab,
line feed and carriage return are removed as a browser removes them, is scheme-relative: `//host`, and
`\\host`, `/\host` and `\/host` because a browser reads a backslash there as a slash. It is not written.
A link to another host keeps its scheme, `https://host`, where whoever reads the document sees it;
`/path`, `?q` and `#frag` stay, since each resolves against the page itself.

**The list is closed in both senses.** It is a list of what is allowed rather than of what is refused,
and the member takes the document and nothing else — no options bag, no policy argument — so a
deployment cannot hold a hole its own configuration opened. An element a real application needs is
added to the list in a commit that says why.

**`Core\Markdown` is the other launderer that rebuilds through this list**
(`rule:core-classes/markdown-to-html`). Its renderer writes only what the list holds, and a document's
raw HTML is rebuilt here. So the list grows by the five value-checked entries Markdown's output needs,
and a Markdown answer sanitized again is unchanged. Neither member has a list of its own.

The acceptance property is **mutation XSS**: sanitizing an answer again changes nothing, which is what
says the approved document is the one a browser will build.
