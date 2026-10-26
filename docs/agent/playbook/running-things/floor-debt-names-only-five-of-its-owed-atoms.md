- **`floor-debt`'s log names only five of the atoms it left owed, and a session cannot run `bun nv
  debt` itself.** The guard refuses it as a heavy run. To list the rest, call `debtOf(new
  SelectStore().atoms())` from a scratch script under `.agent-tmp/`, and pay each one with `bun nv
  proofs --only <feature> --run --no-cache`. A proof that its run reports as `diverged` stays owed
  until the next run of the same proof agrees, so read why it diverged before you run it again. See
  `tools/nv/cmd/debt.ts`. `until: gone tools/nv/cmd/debt.ts:left.slice(0, 5)`
