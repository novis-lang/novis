```nvs
#[Command(name: "deploy", about: "Push the current build")]
public static function deploy(
    string $target,
    #[Option(short: "n", about: "Print what would happen")] bool $dryRun,
    #[Option] uint $retries = 3,
): uint
```

`#[Command]` builds the command table while compiling, over the same program enumeration
`rule:routing/routes-are-compiled-not-registered` uses for the route table
(`rule:programs/implementing`). The three errors every run-time argument parser discovers at the
worst moment are **compile errors** here: a duplicate command name, two options of one command sharing
a short or long spelling, and a parameter whose declared type has no conversion from `string`. That
last one holds for an `#[Option]` and a positional parameter alike (`E0746`). A `#[Command]` method is
static and returns `void` (exit status 0) or `uint`.

The types an argument converts to are the route conversions
(`rule:security/route-capture-is-laundered-by-its-type`), one list the route table reads too, plus a
`bool` `#[Option]` as a flag. **`Core\Time`'s `Date`, `TimeOfDay`, `Instant` and `DateTime` are on it**,
each read from the ISO text its `fromIso` reads, by the same grammar a route capture converts with. A
`DateTime` word needs its `[Zone]`, and is allowed here although a path capture may not declare one,
because no door refuses a `/` in an argument. A word that does not convert is a usage error that names
the form it must have, as for `decimal` and `Core\Uuid`.

A spelling no command line can type is `E0879` at the field: a `name:` or a `long:` that is empty or
holds white space, and a `short:`, the one-letter spelling, that is not one ASCII letter or digit.

A program with no `#[Command]` builds no table, runs no scan and pays nothing. Nobody writes usage
text and nobody lets it rot, because help and completions are read off the table
(`rule:tooling/command-run-dispatches-and-help-is-generated`). What each parameter becomes on the
command line is `rule:tooling/a-parameter-is-an-argument-unless-it-is-an-option`. The alternative — a
`clap`-shaped builder registered at run time — loses all three compile errors and the generated
completions, and is the design every other ecosystem already has.
