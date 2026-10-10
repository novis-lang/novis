- **A socket a container publishes reaches the WSL leg only under `/mnt/wsl`, and on Docker Desktop
  the daemon's own `/mnt/wsl` is not that tmpfs.** The daemon runs in the `docker-desktop` distro,
  whose mount of the tmpfs every
  distro shares is `/mnt/host/wsl`, so a bind source spelled `/mnt/wsl/...` from Windows lands where
  the leg never looks: `wsl.exe
  -d docker-desktop -- ls /mnt/wsl /mnt/host/wsl` beside the leg's own `ls /mnt/wsl` is the tell.
  Docker creates a missing source root-owned 755 and the `redis` image binds as uid 999 whatever
  `user:` says, so the `certs` service chowns it.
  [until: gone tests/db/compose.yaml:/mnt/host/wsl]
