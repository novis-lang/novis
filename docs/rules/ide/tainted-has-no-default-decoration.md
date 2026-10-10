`tainted` gets a semantic-token modifier and **no default decoration**. A marker glyph is added
content rather than a colour, and shipping one on by default writes into someone else's editor exactly
what the token-modifier rule refuses. The marker is `nvs.taint.mark`, and `off` is the default:

| Value | What is marked |
|---|---|
| `off` | nothing; the token modifier alone, styled by the theme |
| `declaration` | a glyph after each declaration whose type carries `tainted` |
| `sink` | `declaration`, plus each argument position where a `tainted` value reaches a sink under `rule:security/sink-predicate` |

Only a declaration is marked, never a use: a `tainted` value is tainted at every use, so a glyph per
use would mark most of a request handler and teach nothing. Where the marker is on, the glyph is one
plain text character in the editor's own font rather than an emoji, which renders at the mercy of the
installed fonts and can show as a replacement box; its colour is a theme reference rather than a
literal. It is not an icon image, because an inline image takes no theme colour and would ship a colour
of Novis's own (`rule:ide/novis-ships-names-not-colours`). Until the sink classification exists on the
`Core` member rows, `sink` marks what `declaration` does.

The cost is a setting that ships inert by default, and that still costs documentation and a test.

**The asymmetry with `secret` is the whole content of this rule.** A credential on a shared screen is
a security incident, which is what buys `secret` its default
(`rule:ide/redaction-ranges-come-from-the-server`). A tainted value on a screen is not an event
at all: `tainted` is a compile-time guarantee already enforced by refusing the sink
(`rule:security/sink-predicate`), so marking it is teaching, and teaching does not get to override the
user's theme.

**Declined:**

- **The marker on by default.** It overrides the user's theme for something that is not a security
  event, and since every `Core\Request` accessor returns `tainted`, most string expressions in a request
  handler would carry it. The default would bury the file, and the setting that turns it off would take
  the `secret` redaction with it in the user's mind.
