- **`wsl -e sh /mnt/e/...` from Git Bash fails with `cannot open C:/Program Files/Git/mnt/e/...`.**
  MSYS rewrites any argument that starts with `/` into a Windows path before `wsl` sees it. Prefix
  the call with `MSYS_NO_PATHCONV=1`, as a local Miri run from the WSL mirror needs
  (`tools/nv/accept/mirror.ts`).
