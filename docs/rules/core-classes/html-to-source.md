`Core\Html::toSource(Core\Html\Markup $markup, string $reason): string` hands back the markup's
source text, and it is the only way out. There is **no `Markup as string` conversion**, because one
would reopen the hole in a keystroke: an escape whose result is immediately cast back to `string` and
concatenated with tainted text is the original bug with an extra word in it.

The shape is the project's standing escape-hatch form — rare, greppable, and carrying a written
reason at the site rather than a silent cast, the same shape `rule:core-classes/secret-reveal` takes.
`$reason` must be a string literal written at the call, and an empty one is refused: a reason that can be computed is a reason
nobody wrote.

Its legitimate callers are the ones that need the bytes and not the guarantee — caching a rendered
fragment, storing one in a column, writing one to a file, handing one to a sink that is not this one.

The name is deliberate. `to…` is the conversion verb, `source` is spelled out, and it is neither
`raw` — which in every template language means the opposite direction — nor `unescape`, which is
reserved for the operation that actually inverts `escape` and which this is not.
