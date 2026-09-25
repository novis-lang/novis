- **A `-p nvs-stdlib` test of a member with an options block must fill the options the way a call
  site does, and `Value::null()` is not that.** `Core\Arr::diff` takes five arguments, and
  `on_of` reads `on` as a plain int (`0`, `1`, `2` for `Core\SetOn`'s three cases), so passing null
  for the default is a `FATAL` that reads exactly like the member refusing the two arrays. Read the
  option's `Const::` default in the registry row's `CoreOption` list before building the argument
  slice — a `Const::EnumCase` arrives as `Value::int(<case index>)` and only a `Const::Null` one
  arrives as `Value::null()`. [until: gone crates/nvs-stdlib/src/arr.rs:Const::EnumCase]
