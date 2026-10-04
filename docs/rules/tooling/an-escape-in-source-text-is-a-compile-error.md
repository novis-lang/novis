An `ESC` byte in a string literal in the source is a compile error naming the site and the fix: *raw terminal
escapes are not how output is styled — use `Core\Cli\Text`.*

This is what keeps `rule:tooling/terminal-output-is-a-sink` from being a trap. A program whose
string literals hold no `ESC` — every program that does not hand-roll colour — is byte-identical with
the sink in place, and the one program that would be surprised by `␛[31m` on its screen is told at
compile time, at the exact line. With the written case closed, the only way to meet the substitution at run
time is to print a **computed** escape sequence, which is the attack the sink exists to neutralize.
The diagnostic names `Cli\Text` so that a developer who wants colour finds the one raw path
(`rule:tooling/text-is-the-one-raw-path`) instead of concluding that colour does not work.
