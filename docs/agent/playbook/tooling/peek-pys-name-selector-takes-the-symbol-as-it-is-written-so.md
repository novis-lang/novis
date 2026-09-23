- **`peek.py`'s `:@name` selector takes the symbol as it is *written*, so a `Type::method` spelling
  lands somewhere else or nowhere.** `object.rs:@ClassTable::define` printed `set_field_tags`, whose
  doc comment merely *mentions* `ClassTable::define`, and `@NvsObj::set_field` answered "no
  definition or mention" for a method three screens down — an inherent method is written
  `pub fn set_field`, and the `impl` block's name is not part of the token. Ask for the bare name
  (`@set_field`), or `--locate set_field`, and read the `impl` off the anchor.
  [until: reviewed 2026-09-08]
