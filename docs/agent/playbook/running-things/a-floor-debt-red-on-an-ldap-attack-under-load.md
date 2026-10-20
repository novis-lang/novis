- **A `floor-debt` red on one `requires:` program can be the full run's load, and running it alone
  pays it.** `bun nv debt` runs every owed proof program in one 45-minute run, and an LDAP attack
  under `tests/hostile/core/Ldap-Connection/` missed its socket deadline there. `bun nv proofs --run
  --id '<feature>'` runs it alone on the recording build and records its atom green in
  `.cache/select.sqlite`, so the next `floor-debt` has nothing left to pay. The guard refuses
  `bun nv debt` in a session. `until: gone tools/nv/cmd/debt.ts:export const DEBT_ITEM`
