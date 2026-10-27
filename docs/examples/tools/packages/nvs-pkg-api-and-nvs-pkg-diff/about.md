`nvs pkg api <dir>` prints the public API of a package as JSON. The public API is every public class,
interface, enum and type, with their methods, properties and constants. The command checks the
files under `<dir>/src`, and it does not run them.

`nvs pkg diff <old> <new>` compares two versions of a package. Each change prints on one line,
as `breaking` or `additive`. A breaking change can stop code that uses the package from compiling.
A removed method is breaking, for example, and a new optional parameter is additive.

The version numbers decide which changes are allowed. Each version comes from `package.toml`, and
`--from` and `--to` replace it. A major release, such as `1.2.0` to `2.0.0`, may change anything. A
minor or patch release may only add. A `0.0.x` version is experimental, so nothing is checked. The
command exits with `1` when a change is not allowed.

**Good to know:** the check is about the shape of the API only. The behaviour of a method may still
change in any release.
