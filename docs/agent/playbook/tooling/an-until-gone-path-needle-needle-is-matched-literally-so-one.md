- **An `[until: gone <path>:<needle>]` needle is matched literally, so one that drops the source's
  backticks fires the moment it is written.** `session.py --wrap` retired a `carried-gaps.md` bullet
  in the same call that added it, because the needle read `bytes round trip` where
  `crates/nvs-stdlib/src/uuid.rs` writes `` `bytes` round trip ``. Copy the needle out of the file it
  watches, or point it at a plain-text state such as `owner: unowned` instead.
  [until: gone tools/nv/cmd/playbook.ts:holds]
