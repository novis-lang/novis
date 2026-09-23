- **`echo` of a control character prints its picture, not the character.** A case that echoes `"\r"`,
  `"\v"`, `"\f"` or `"\e"` to show what an escape produced gets `␍ ␋ ␌ ␛` back, because the terminal
  sink substitutes the U+240x picture for a control character, so an `--EXPECT--` written from a run
  pins the picture rather than the escape. Assert an escape by comparing it with the same code point
  written the long way — `"\r" == "\u{D}"` — or by `Core\Str::length`, and keep control characters out
  of the expected output entirely. [until: reviewed 2026-09-19]
