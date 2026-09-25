- **A `--EXPECT--` is byte-exact, and a loop that echoes its separator *after* each item leaves a
  trailing space nothing shows you.** `echo $a, "/", $b, " ";` inside a `foreach` costs a run: the
  expected block cannot carry a trailing space (an editor or a hook strips it, and the diff prints
  identically on both sides). Collect the rows into an `array<string>` and echo
  `Core\Str::join($rows, " ")`; a fold member's `int|float|decimal` answer concatenates with `.`
  even where `as string` on that union does not. [until: gone crates/nvs-stdlib/src/str.rs:Core\Str::join]
