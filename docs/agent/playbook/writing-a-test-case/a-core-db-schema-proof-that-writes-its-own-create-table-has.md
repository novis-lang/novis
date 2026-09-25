- **A `Core\Db\Schema` proof that writes its own `create table` has to spell each column type the way
  the dialect emitter writes it, or the plan opens with a whole-table rebuild.** A column declared
  `title text not null` against a schema saying `text(200)` is a *type change*, which SQLite cannot
  alter in place, so the diff grades it `Destructive` and a proof written to show one `Safe` step
  prints a create-copy-drop-rename in front of it. Write `varchar(200)`, and read the SQL of step one
  out of `planAgainst` once before blessing anything. [until: gone crates/nvs-stdlib/src/db/schema.rs:Core\Db\Schema]
