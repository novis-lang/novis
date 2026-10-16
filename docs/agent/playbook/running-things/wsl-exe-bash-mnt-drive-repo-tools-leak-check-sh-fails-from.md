- **`wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh …` fails from Bash and still exits 0.**
  Git Bash rewrites any argument that looks like a POSIX path before `wsl.exe` sees it, so the command
  arrives as `bash: C:/Program Files/Git/mnt/…: No such file or directory`, which reads as a clean
  leak check on a fixture that never ran. Put `MSYS_NO_PATHCONV=1` in front of the line; any absolute
  POSIX path handed to a Windows `.exe` through Git Bash needs it.
  `until: gone tools/leak-check.sh:/var/tmp/nvs-linux`
