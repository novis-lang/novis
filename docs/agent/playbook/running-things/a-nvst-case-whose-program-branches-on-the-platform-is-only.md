- **A `.nvst` case whose program branches on the platform is only half-run by
  `target/debug/nvs.exe`.** `/var/tmp/nvs-linux/debug/nvs` is a Linux binary the valgrind leg keeps
  current, so `wsl.exe -- bash -lc "cd /mnt/<drive>/<repo> && /var/tmp/nvs-linux/debug/nvs test
  <case>.nvst"` runs the case on Linux with no build. Check its date first: it is as old as the last
  valgrind sweep and has none of this session's Rust. [until: gone tools/leak-check.sh:/var/tmp/nvs-linux/debug/nvs]
