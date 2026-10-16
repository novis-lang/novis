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
a short or long spelling, and an `#[Option]` on a parameter whose declared type has no conversion from
`string`. A `#[Command]` method is static and returns `void` (exit status 0) or `uint`.

A program with no `#[Command]` builds no table, runs no scan and pays nothing. Nobody writes usage
text and nobody lets it rot, because help and completions are read off the table
(`rule:tooling/command-run-dispatches-and-help-is-generated`). What each parameter becomes on the
command line is `rule:tooling/a-parameter-is-an-argument-unless-it-is-an-option`. The alternative — a
`clap`-shaped builder registered at run time — loses all three compile errors and the generated
completions, and is the design every other ecosystem already has.
