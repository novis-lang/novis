- **`python tools/try.py` ignores `--ENV--`, so an env-dependent case runs against the machine's own
  environment.** The scratch runner honours `--FILE--` and the expectations but never sets the
  section's variables, so a `Core\Env` case comes back `whole: 0 of 4` and reads as a broken member.
  Check an `--ENV--` case with `target/debug/nvs.exe test <path/to/one-case.nvst>`, which takes a
  single file as well as a tree. [until: exists tools/try.py:--ENV--]
