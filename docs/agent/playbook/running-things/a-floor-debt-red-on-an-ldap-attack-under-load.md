- **An LDAP attack that times out, or stays owed in `floor-debt` with 0 red, can be Samba's restart
  back-off.** Some attacks crash a Samba LDAP worker, and Samba waits longer after each crash to
  restart one. `docker logs novis-db-samba-ad-1` shows `terminated with signal 11`. `tests/db/compose.yaml`
  sets the back-off to 1 s, and an older container needs `docker compose up -d --wait samba-ad`. Then `bun nv proofs --run
  --id 'Core\Ldap\Connection::<method>'` records the atom green. The guard refuses `bun nv debt`.
  `until: gone tests/db/compose.yaml:prefork maximum backoff`
