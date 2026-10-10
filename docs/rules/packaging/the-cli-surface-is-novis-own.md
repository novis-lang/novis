**No flag on `nvs` exists because another tool spells it that way.** A spelling inherited from
elsewhere buys a few users one familiar keystroke and charges every reader of `--help` for it
afterwards.

The whole short-flag surface, and where each one comes from:

| Spelling | Source |
|---|---|
| `-h`, `--help` | clap, generated |
| `-V`, `--version` | clap, from `version` in the `#[command]` attribute |
| `-o`, `--output` | ours, on `nvs build --compile` alone |

`-h` and `-V` stay, and are not the thing this rule is about: they are the Unix baseline every program on
the machine shares, and `-V` is uppercase because the GNU convention reserves `-v` for verbosity.

A short flag is a decision, not a convenience. Declare one only where a long name is genuinely typed
often enough to hurt — `-o` is that, and nothing else in the CLI has met the bar. **A short and its long
form are one flag with two spellings**, which is the getopt convention and not an operation reachable two
ways; `rule:core-api/one-paradigm-per-operation` governs `Core`'s surface and never had anything to say
about argv.

What a program observably does and what the command line looks like are separate questions: argv is
not program semantics, so a migrating PHP developer's habits are no argument for a flag. A PHP
spelling typed by habit, such as `nvs -i`, gets clap's unknown-argument error, which is one extra
keystroke discovered once.

**Declined:**
- **`nvs -i` as an alias of `nvs info`.** It needs a global flag, a second global `--licenses` that
  cannot be typed alone, and a hand-written check that a subcommand was not also given, because clap
  cannot name a subcommand in a conflict. Three mechanisms for one spelling
  (`rule:packaging/nvs-info-is-the-one-call`).
- **Keeping `-i` and dropping only the derived `--info`.** The flag is what carries all three
  mechanisms, so the spelling was never the expensive half.
- **`-v` for the version, as PHP spells it.** It would be a flag chosen because PHP spells it that way,
  and it spends the letter every future `--verbose` will want.
