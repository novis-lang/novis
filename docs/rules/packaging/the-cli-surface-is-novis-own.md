**No flag on `nvs` exists because PHP spells it that way.** The language copies PHP's observable
semantics on purpose; the command line is not semantics, and a spelling inherited there buys a PHP
developer one familiar keystroke and charges every reader of `--help` for it afterwards.

The whole short-flag surface, and where each one comes from:

| Spelling | Source |
|---|---|
| `-h`, `--help` | clap, generated |
| `-V`, `--version` | clap, from `version` in the `#[command]` attribute |
| `-o`, `--output` | ours, on `nvs build --compile` alone |

`-h` and `-V` stay, and are not the thing this rule is about: they are the Unix baseline every program on
the machine shares, and `-V` is uppercase because the GNU convention reserves `-v` for verbosity. PHP's
own version flag is the lowercase `-v`, so adopting it would be the one change that moved *toward* PHP.

A short flag is a decision, not a convenience. Declare one only where a long name is genuinely typed
often enough to hurt — `-o` is that, and nothing else in the CLI has met the bar. **A short and its long
form are one flag with two spellings**, which is the getopt convention and not an operation reachable two
ways; `rule:core-api/one-paradigm-per-operation` governs `Core`'s surface and never had anything to say
about argv.
