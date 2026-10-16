`Core\Html::sanitize` answers `Core\Html\Markup`, which makes it the fourth way to obtain one and the
only one that takes a runtime-computed string.

That is exactly why it must be a parser that **rebuilds the document from a known-good grammar**, and
never a filter that deletes what looks dangerous. A filter answering a carrier would be a generic
sanitizer wearing a type — it would claim a guarantee it cannot establish, because "what looks
dangerous" is a list an attacker gets to extend.

**The rebuild is three steps and only the middle one holds a policy.** The document is parsed by the
WHATWG algorithm, which cannot fail; a new tree is built from a closed list of elements, each with the
attributes it may carry; and that tree is written back out under the serialization rules of the door it
came through. An element the list does not hold contributes no tag — dropped with its content when its
content model is raw text, since a `<script>`'s character data is neither markup nor prose, and
unwrapped otherwise, because `html`, `head` and `body` are elements the algorithm inserts around every
fragment. An attribute the list does not grant is not written, and a `href` or `src` naming a scheme
outside the ones that fetch is not written either.

**The list is closed in both senses.** It is a list of what is allowed rather than of what is refused,
and the member takes the document and nothing else — no options bag, no policy argument — so a
deployment cannot hold a hole its own configuration opened. An element a real application needs is
added to the list in a commit that says why.

The acceptance property is **mutation XSS**: sanitizing an answer again changes nothing, which is what
says the approved document is the one a browser will build.
