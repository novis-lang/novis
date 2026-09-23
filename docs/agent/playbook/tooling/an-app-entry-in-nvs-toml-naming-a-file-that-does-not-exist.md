- **An `[[app]]` entry in `nvs.toml` naming a file that does not exist aborts every program in the
  tree with `E0605`, not just that one.** A key that matches nothing would silently drop the
  application it was meant to cover back to the global configuration, so the loader refuses the
  whole file — which means writing a proof's grants before writing the proof makes every other
  example and fixture unrunnable in the meantime. Write the `.nvs` files first and the grant block
  second, and when a run fails on a path you have not created yet, that is what happened.
  [until: reviewed 2026-09-19]
