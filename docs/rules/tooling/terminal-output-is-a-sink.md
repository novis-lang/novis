Standard output and standard error are sinks. Every value written through `echo` or `Cli::write` —
**regardless of qualifier** — has its control bytes replaced before a byte reaches the stream: `LF`
and `TAB` pass through; every other C0 byte, `ESC` and `CR` included, becomes its U+2400 Control
Picture (`ESC` → `␛`, `CR` → `␍`); `DEL` becomes `␡`; a C1 code point becomes `�`; an unterminated
bidirectional control becomes `�` while a balanced one passes (`rule:security/bidi-predicate`). Twelve
terminal CVEs in 2022–23 were reached through exactly the output `git`, `less` and `kubectl` print.
`CR` is on the list because a bare `CR` overwriting its own line is the oldest way to hide text, and
it needs no `ESC`. A C1 code point loses its identity because several terminals still parse one as a
CSI introducer, the U+2400 block has no glyph for it, and it is never legitimate text.

The rule is **uniform**. It does not depend on whether the value is `tainted`, because that would make
*whether output is escaped* hinge on a fact invisible at the `echo` line; and it does not depend on
whether the stream is a terminal, because a CI log is written to a pipe and read by a human later.
Styling is tty-dependent (`rule:tooling/the-terminal-profile-resolves-once`); neutralizing never is.
In one sentence: **`echo` prints text. `Core\Cli\Text` prints terminal commands.**

Unlike HTML's `&` → `&amp;`, this transforms no visible text: a control sequence was never text, and
today those bytes vanish into the terminal's command stream unseen. Substituting makes `echo` show
*more* of what arrived, and nothing is silently dropped. `Core\Cli::escape(tainted string): string`
is the sink's named launderer, performing exactly this table, for the program that wants the
neutralized form as a value. A `secret` value is refused outright with no carrier bypass
(`rule:security/secret-sinks-refuse`), because substitution does nothing for confidentiality.

**Memory:** nothing is held. The substitution is one scan over bytes already being written.

**Declined:**
- **Deleting control bytes rather than substituting them.** It is silently lossy, and it gives up the
  property that makes the default defensible: the rule shows itself in the output.
- **Refusing `tainted` at the sink and requiring an explicit launderer.** It puts friction on the most
  common CLI operation, printing data just read, and that friction trains a developer to reach for a
  blanket launderer. With no launder step on the ordinary path, there is no omitted call to get wrong.
