`Cli\Text` is peer to `Core\Html\Markup` and the only value the terminal sink writes raw. Its two
constructors, `Text::plain(string)` and `Text::styled(string, Style)`, **both apply the sink's
substitution to their input**, so the only control bytes a `Text` can carry are the ones its `Style`
put there. That is the structural guarantee: `Text` is not a trust assertion a developer can be
tricked into making, it is a constructor that cannot produce an injected sequence. `Text + Text` is
`Text`, immutable, composing the way `Markup + Markup` does, and `Text::plain` is idempotent, so text
that has already been through the sink is never escaped twice.

A `Text` holds bytes, so the styling is rendered when it is built, against the profile resolved once
for the process (`rule:tooling/the-terminal-profile-resolves-once`), not at the write. The known
limit: a `Text` written to a terminal standard output and to a redirected standard error in one run
sends both the same bytes. Closing it would make the sink's carrier
(`rule:security/capture-answers-the-carrier`) two representations instead of one, and that trade is
not worth making while `echo` writes standard output and nothing else can observe the difference.
