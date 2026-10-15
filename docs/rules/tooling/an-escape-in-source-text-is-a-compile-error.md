An `ESC` byte in a string literal in the source is a compile error, `E0011`, at the character or escape
that writes it, and its message names the fix: style terminal output with `Core\Cli\Text`.

Every way a literal can write the byte is caught. In a double-quoted string, an interpolated one, a heredoc
and an html template segment, the check runs over what each character or escape decodes to, so a raw byte,
`\e`, `\x1b`, an octal escape such as `\033` and `\u{1b}` are each one error
(`crates/nvs-syntax/src/string_lit.rs`, `CookIssue::EscapeCharacter`). A single-quoted string and a nowdoc
apply no such escapes, so the checker scans their text for the raw byte
(`crates/nvs-types/src/expr/literals.rs`, `check_raw_escape_characters`). A heredoc whose closing marker is
indented underlines the whole run, the same loss of precision its other escape errors have. There is no
`bytes` literal, so `"…" as bytes` is held to the same check, and a program builds a byte string holding
`ESC` at run time.

This is what keeps `rule:tooling/terminal-output-is-a-sink` from being a trap. A program whose
string literals hold no `ESC` — every program that does not hand-roll colour — is byte-identical with
the sink in place, and the one program that would be surprised by `␛[31m` on its screen is told at
compile time, at the exact line. With the written case closed, the only way to meet the substitution at run
time is to print a **computed** escape sequence, which is the attack the sink exists to neutralize.
The diagnostic names `Cli\Text` so that a developer who wants colour finds the one raw path
(`rule:tooling/text-is-the-one-raw-path`) instead of concluding that colour does not work.
