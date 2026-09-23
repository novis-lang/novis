- **`wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh …` fails from the Bash tool and works
  from PowerShell.** Git Bash rewrites any argument that looks like a POSIX path before `wsl.exe`
  sees it, so the command arrives as `bash: C:/Program Files/Git/mnt/…: No such file or directory` —
  and still exits 0, which reads as a clean leak check on a fixture that never ran. Run the
  identical line through the PowerShell tool, or put `MSYS_NO_PATHCONV=1` in front of it; any
  absolute POSIX path handed to a Windows `.exe` through Git Bash is a candidate.
  [until: reviewed 2026-09-06]
