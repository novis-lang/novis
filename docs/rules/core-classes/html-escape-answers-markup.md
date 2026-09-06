`Core\Html::escape(tainted string $text): Core\Html\Markup` answers the carrier its sink accepts, not
a `string`. The bytes it produces are unchanged; only the wrapper is new.

**The eager-escape habit stops compiling**, which is the point. A developer arriving from
`htmlspecialchars()` writes the escape call, then finds the result cannot be concatenated back into
the surrounding string, because `.` has no row for a carrier. They are corrected at the call site
instead of shipping `&amp;amp;`, and for most code the correction is to delete the escape call
entirely — the sink was always going to do it (`rule:core-classes/html-auto-escape`).

`escape` additionally **neutralizes an unterminated bidirectional control**, substituting a
replacement character. Escaping `<`, `>`, `&` and quotes does nothing about display order, so without
that rule a bidi payload would survive the auto-escape sink intact. A balanced control is legitimate
mixed-direction text and passes through.
