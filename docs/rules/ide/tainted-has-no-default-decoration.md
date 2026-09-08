`tainted` gets a semantic-token modifier and **no default decoration**. A marker glyph is added
content rather than a colour, and shipping one on by default writes into someone else's editor exactly
what the token-modifier rule refuses. The marker is a setting with three values and `off` is the
default; where it is on, the glyph is a themed icon rather than an emoji, and its colour is a theme
reference rather than a literal.

**The asymmetry with `secret` is the whole content of this rule.** A credential on a shared screen is
a security incident, which is what buys `secret` its default
(`rule:ide/redaction-ranges-come-from-the-server`). A tainted value on a screen is not an event
at all: `tainted` is a compile-time guarantee already enforced by refusing the sink
(`rule:security/sink-predicate`), so marking it is teaching, and teaching does not get to override the
user's theme.

**Not on disk.** There is no language server in the tree.
