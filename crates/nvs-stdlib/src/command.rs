//! `Core\Command` — the compiled command table, as the members that read it.
//!
//! `rule:tooling/commands-are-compiled`
//! builds the table *while compiling*, from the `#[Command]` methods the ADR
//! 0061 § 3 scan found, and this class is the whole of what a program does with
//! it. `nvs_runtime::commands` is the table as a running program holds it, and
//! its module doc owns why the rows cross the boundary at all rather than being
//! folded while checking — the short version is that `help`'s `?string $name`
//! and `run`'s argument vector are both runtime values.
//!
//! # What a page looks like, and why that is decided here
//!
//! § 6 says the usage page is *generated* and says nothing about its layout, so
//! the layout is this module's call and this is its one home:
//!
//! ```text
//! usage: greet <name> [--loud] [--times]
//!
//! Greet somebody by name
//!
//! arguments:
//!   <name>  string  Who to greet
//!
//! options:
//!   --loud   bool      Shout it
//!   --times  uint = 3  How many times
//! ```
//!
//! A **positional argument** is `<param>` and an **option** is `[--spelling]`,
//! which is the convention every CLI in the audience's world already reads
//! (`rule:programs/audience`) —
//! and it is read off the row's own `spellings` rather than off a second field,
//! for the reason `nvs_runtime::commands::CommandArg::spellings` states. An
//! option that declares both a short and a long spelling is *summarized* by its
//! long one in the usage line and lists all of them in the `options:` block: one
//! line that names every spelling is unreadable at four options, and the block
//! below it is where the full answer belongs. Each block aligns its own
//! columns, so a long spelling under `options:` does not indent `arguments:`.
//!
//! The middle column is the **declaration** — the parameter's type, and
//! `= <value>` after it where the declaration wrote a default. It is the one
//! column the table cannot answer out of itself: a row carries what a parameter
//! is *called* and the conversion the checker picked for it, never the type it
//! was declared at (`nvs_types::commands` § *What crosses in a row, and what
//! stays*, where that is a stated bound). The page reaches the handler's own
//! signature instead, through the `Class::method` label the row already holds —
//! [`nvs_runtime::MethodRow::param_types`], found by the parameter's name — so
//! there is one source of truth and no second copy of a signature in the table.
//!
//! **Both halves of a declaration or neither**, which is why the default was
//! withheld until the type could be reached: a page naming `= 3` and no type
//! reads as though the declaration had written none. So a page rendered with no
//! way to reach the class table, and a parameter whose handler's row no
//! declaration was read for, print no declaration column at all rather than
//! half of one.
//!
//! A **flag** names its type and never a default. § 6 gives an option declared
//! `bool` by its being *written*, so the one nobody wrote is `false` whatever
//! its declaration says (`nvs_runtime::commands::CommandArg::default`), and
//! `bool = true` on a page would name a default the matcher does not honour.
//!
//! `help(null)` is the program's own page — the command list rather than one
//! command's arguments — because a CLI's bare `--help` names its subcommands,
//! and a program that declares none says so in a sentence rather than printing
//! an empty heading.
//!
//! The page ends with a newline, so `echo Core\Command::help(…)` is a page and
//! not a page welded to the next thing written. A caller who wants the lines
//! splits it (`Core\Str::lines`), which is what `examples/cli.nvs` does.
//!
//! # What a completion script completes, and why that is decided here
//!
//! § 6 names the four shells and says nothing about what the script does, so
//! that is this module's call too, and this is its one home. **Every script
//! completes two positions and no others**: the command word, from the table's
//! own names, and the option spellings of whichever command the first word
//! already selected. It completes no *value* — not a positional argument's and
//! not an option's. A declared type is not a set of values: the page can say
//! `uint` and a script would have to offer a number, so what it offered would
//! be invented rather than generated. **That is a bound and not an omission**,
//! and it holds for the two conversions that *do* carry their words — a union
//! of literals and an enum's cases, `nvs_runtime::commands::ArgConv::OneOf` and
//! `ArgConv::Enum` — because a script completing some values and staying silent
//! about others teaches its user that the ones it left out are wrong.
//!
//! Each is written in the shell's own idiom rather than in a common shape bent
//! four ways: a `bash` function over `COMP_WORDS` behind `complete -F`, a `zsh`
//! `#compdef` function using `_describe` then `_arguments`, one `complete -c`
//! line per command and per spelling for `fish`, and a
//! `Register-ArgumentCompleter -Native` block for PowerShell. A script that
//! reads as though a human wrote it is one its user can edit, and the four
//! shells agree on too little for a shared skeleton to be worth its
//! indirection.
//!
//! **The name every script registers against is the program's**, which is the
//! one fact the table does not carry: [`nvs_runtime::Ctx::program_name`] owns
//! which name that is for each way a program can be started, and why
//! `nvs-stdlib` is handed it rather than reading `argv[0]` (`rule:security/capability-check-at-the-door`).

use nvs_runtime::commands::{ArgConv, CaseValue, Command, CommandArg, CommandTable};
use nvs_runtime::{Decimal, Fault, MethodRow, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// This class's fully-qualified name, in one place so the registry row and
/// every refusal that names the class cannot drift apart.
pub(crate) const NAME: &str = r"Core\Command";

/// `rule:tooling/commands-are-compiled`'s generated help, as a registry row. See
/// [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "help",
            names: &["name"],
            // `Qual::Neutral` and not [`Qual::Sink`]: the name is matched against
            // the table and never becomes an instruction, and it arrives `tainted`
            // in the one call that matters — a command line's own first word.
            params: &[CoreTy::Nullable(&CoreTy::Text(Qual::Neutral))],
            defaults: &[],
            return_ty: CoreTy::Instance(crate::cli::NAME),
            symbol: "nvs_core_command_help",
            doc: Some(&HELP_DOC),
        },
        CoreMethod {
            name: "run",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_command_run",
            doc: Some(&RUN_DOC),
        },
        CoreMethod {
            name: "completions",
            names: &["shell"],
            params: &[CoreTy::Enum(crate::cli::SHELL_NAME)],
            defaults: &[],
            return_ty: CoreTy::Text(Qual::Neutral),
            symbol: "nvs_core_command_completions",
            doc: Some(&COMPLETIONS_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Command::help`'s reference card — `rule:core-api/reference-card`.
const HELP_DOC: MethodDoc = MethodDoc {
    short: "The usage page, generated from the table `#[Command]` built while compiling — one \
            command's arguments, or the program's own list of commands. Nobody writes usage text \
            and nobody lets it rot.",
    params: &[ParamDoc {
        name: "name",
        desc: "The command to describe, or `null` for the program's own page listing every \
               command it declares.",
        shape: &[],
    }],
    ret: "The rendered page as a `Core\\Cli\\Text`, ending with a newline.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The program declares no `#[Command]` under `$name` — a page for a command that \
               does not exist is a mistake in the program rather than in its input, since the \
               table is fixed at compile time.",
    }],
};

/// `Core\Command::run`'s reference card — `rule:core-api/reference-card`.
const RUN_DOC: MethodDoc = MethodDoc {
    short: "Matches this process's own command line against the table `#[Command]` built while \
            compiling, calls the handler the first word names, and answers the status the process \
            should exit with.",
    params: &[],
    ret: "The handler's own `uint`, or `0` where it is declared `void`. A command line this \
          program's table does not answer is a **usage error**: the page goes to standard error \
          and the status is `2`, which is the status a command line nobody can act on has meant \
          since `getopt`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The named command's handler is not a method this program declares, or one of its \
               parameters is declared at a type no argument's text is converted into yet — both \
               are mistakes in the program rather than in the command line it was given.",
    }],
};

/// `Core\Command::completions`'s reference card — `rule:core-api/reference-card`.
const COMPLETIONS_DOC: MethodDoc = MethodDoc {
    short: "A completion script for this program, in the named shell's own syntax, generated from \
            the same table `help` reads: every command it declares, and every option each one \
            takes.",
    params: &[ParamDoc {
        name: "shell",
        desc: "Which shell to write the script for.",
        shape: &[],
    }],
    ret: "The script as plain text, ending with a newline — something to redirect into the shell's \
          completion directory rather than something to print at a terminal.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program has no name for the script to register against, which is every \
               context but a command-line run: a served request is not something a shell \
               completes.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_command_help" => (nvs_core_command_help as *const ()).cast(),
        "nvs_core_command_run" => (nvs_core_command_run as *const ()).cast(),
        "nvs_core_command_completions" => (nvs_core_command_completions as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Command::help(?string $name): Cli\Text` — replacing the usage text
    /// every argument-parsing package asks its user to write twice.
    ///
    /// The module doc owns the layout. What is here is the lookup: a context
    /// carrying no table and a program declaring no command are one case
    /// (`nvs_runtime::commands`), so both render the same "declares no command"
    /// page and neither is an error.
    fn nvs_core_command_help(ctx, args: [1]) {
        let empty = CommandTable::default();
        let table = ctx.commands().unwrap_or(&empty);
        let page = match args[0].tag() {
            Some(Tag::Null) => overview(table),
            _ => {
                let Some(name) = args[0].as_text() else {
                    // Unreachable from source: the parameter is `?string`, so
                    // `E0401` refuses anything else before this body runs.
                    return Err(Fault::fatal(format!(
                        "Core\\Command::help expected a `?string` name, got tag {}",
                        args[0].tag_byte()
                    )));
                };
                let Some(row) = table.named(name) else {
                    return Err(Fault::thrown_as(
                        ThrownClass::Logic,
                        format!(
                            "`Core\\Command::help` was asked for `{name}`, which this program \
                             declares no `#[Command]` for"
                        ),
                    ));
                };
                page_for(row, signature_of(ctx, row))
            }
        };
        Ok(crate::cli::built(Value::str(NvsStr::new(page.as_bytes()))))
    }
}

/// The exit status a command line nobody can act on answers with — see
/// [`RUN_DOC`]'s `ret`, which is this constant's one home.
const USAGE_STATUS: u64 = 2;

nvs_runtime::nvs_helper! {
    /// `Core\Command::run(): uint` — `rule:tooling/commands-are-compiled`'s entry point, and its one
    /// deliberate divergence from `rule:routing/routes-are-compiled-not-registered`: this table *dispatches*.
    ///
    /// Three steps, in the order a command line is read. The first word selects
    /// a row of the same table [`nvs_core_command_help`] renders; the words past
    /// it fill that row's arguments ([`matched`]); and the handler is reached by
    /// the label the compiler wrote into the row, through
    /// [`nvs_runtime::call_static`], whose docs own why a `Class::method` string
    /// is enough to reach a `static` method and nothing had to be installed
    /// beside the table.
    ///
    /// Anything the *command line* got wrong is a usage error — the page to
    /// standard error and status [`USAGE_STATUS`], never a throw — and anything
    /// the *program* got wrong is a `LogicError`, because the table is fixed at
    /// compile time and a program cannot mistype its way into one at run time.
    fn nvs_core_command_run(ctx, _args: [0]) {
        let line: Vec<String> = ctx.command_line().to_vec();
        let selected = {
            let empty = CommandTable::default();
            let table = ctx.commands().unwrap_or(&empty);
            match line.first() {
                None => Err(overview(table)),
                Some(word) => table.named(word).cloned().ok_or_else(|| {
                    format!("no command is spelled `{word}`\n\n{}", overview(table))
                }),
            }
        };
        let row = match selected {
            Ok(row) => row,
            Err(page) => return usage(ctx, &page),
        };
        // Asked once, before a word is matched, so that a row the compiler
        // could not choose a conversion for answers as what it is — a
        // declaration that was refused where it was written
        // (`nvs_runtime::commands::ArgConv::Unconverted`) — rather than as a
        // complaint about the argument that happened to reach it.
        if let Some(arg) = row.args.iter().find(|arg| arg.conv == ArgConv::Unconverted) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "`Core\\Command::run` has no conversion into the type `{}`'s `{}` is \
                     declared at",
                    row.name, arg.param
                ),
            ));
        }
        let values = match matched(&row, &line[1..]) {
            Ok(values) => values,
            Err(problem) => {
                // The page before the call, not inside its argument list: the
                // signature is a shared borrow of the context `usage` then
                // writes through.
                let page = page_for(&row, signature_of(ctx, &row));
                return usage(ctx, &format!("{problem}\n\n{page}"));
            }
        };
        // The one conversion `matched` leaves undone, because it is the one
        // needing a context rather than only a word. `parse_each` is the home of
        // why it is a second pass and of why a refusal lands on the usage page.
        let values = match parse_each(ctx, &row, values) {
            Ok(values) => values,
            Err(Refused::Usage(problem)) => {
                let page = page_for(&row, signature_of(ctx, &row));
                return usage(ctx, &format!("{problem}\n\n{page}"));
            }
            Err(Refused::Fault(fault)) => return Err(fault),
        };
        // The callee reads one slot per declared parameter, so a count that
        // disagrees with the row is memory-unsafe rather than wrong. This is
        // unreachable from source in a `debug_assert!`'s sense and not in a
        // diagnostic's: `fill` fills every slot or answers `Err`, so no command
        // line can produce a short list — the guard is what makes the call
        // below sound rather than merely believed.
        if values.len() != row.args.len() {
            return Err(Fault::fatal(format!(
                "internal error: `{}` declares {} argument(s) and the matcher filled {}",
                row.name,
                row.args.len(),
                values.len()
            )));
        }
        let outcome = nvs_runtime::call_static(ctx, &row.handler, &values);
        #[expect(
            unsafe_code,
            reason = "each value here is one this frame made and owns; \
                      `call_static` retained its own for the callee to release"
        )]
        unsafe {
            for value in &values {
                value.release();
            }
        }
        match outcome? {
            // A handler is declared `void` or `uint` (`nvs_types::commands`'s
            // `check_command_shape`), so the value is a number or `null` and
            // there is no reference here to release.
            Some(value) => Ok(Value::uint(value.as_uint().unwrap_or(0))),
            None => Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "`{}` is the handler `{}` was declared on, and this program declares no such \
                     method",
                    row.handler, row.name
                ),
            )),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Command::completions(Cli\Shell $shell): string` — § 6's second
    /// generated artefact, over the same table [`nvs_core_command_help`]
    /// renders.
    ///
    /// The module doc owns the four layouts. What is decided here is the one
    /// thing a *page* never needs and a *script* cannot do without: the name the
    /// script registers against, which is [`nvs_runtime::Ctx::program_name`] and
    /// is empty for every context that is not a command-line run. Refusing there
    /// rather than substituting something is the same call `help` makes for a
    /// command that does not exist — the program asked for an artefact this
    /// context cannot produce, which is a mistake in the program.
    fn nvs_core_command_completions(ctx, args: [1]) {
        let shell = shell_of(&args[0])?;
        let program = ctx.program_name().to_owned();
        if program.is_empty() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "`Core\\Command::completions` has no name to register a script against: this \
                 program was not started from a command line, and a served request is not \
                 something a shell completes"
                    .to_owned(),
            ));
        }
        let empty = CommandTable::default();
        let table = ctx.commands().unwrap_or(&empty);
        let script = match shell {
            Shell::Bash => bash_script(&program, table),
            Shell::Zsh => zsh_script(&program, table),
            Shell::Fish => fish_script(&program, table),
            Shell::Pwsh => pwsh_script(&program, table),
        };
        Ok(Value::str(NvsStr::new(script.as_bytes())))
    }
}

/// A usage error: the page on the **diagnostic** channel, and
/// [`USAGE_STATUS`] as the process's status.
///
/// Standard error and not standard output, because a page printed *instead of*
/// what was asked for is not the program's answer — a shell pipeline reading
/// this program's output must not receive a usage page as data. It is the
/// channel `Core\Debug::dump` already writes to (`rule:errors/debug-dump`), which is this
/// tree's only spelling of "not the answer" until `rule:tooling/the-terminal-profile-resolves-once`'s `write` lands.
///
/// # Errors
///
/// A sink that refuses the bytes, which for a CLI run is a closed standard
/// error — the world said no, so `RuntimeError`. It is unreachable from source:
/// nothing a program can write closes the channel it is being reported on, and
/// a `Core\Out::capture` deliberately does not redirect this one.
fn usage(ctx: &mut nvs_runtime::Ctx, page: &str) -> Result<Value, Fault> {
    ctx.write_diagnostic(page.as_bytes())
        .map_err(|error| Fault::thrown(format!("a usage page could not be written: {error}")))?;
    Ok(Value::uint(USAGE_STATUS))
}

/// Every word past the command name, matched onto `row`'s arguments: one value
/// per declared parameter, in declaration order, ready for the handler's frame.
///
/// `Err` is the sentence a usage page leads with, and nothing survives it — a
/// `string` already matched holds the one reference this frame made, so the
/// failing path releases what the succeeding path hands on.
fn matched(row: &Command, words: &[String]) -> Result<Vec<Value>, String> {
    let mut slots: Vec<Option<Value>> = row.args.iter().map(|_| None).collect();
    match fill(row, words, &mut slots) {
        Ok(()) => Ok(slots.into_iter().flatten().collect()),
        Err(problem) => {
            #[expect(
                unsafe_code,
                reason = "a filled slot holds the one reference `convert` made \
                          and this frame still owns, and nothing else has seen it"
            )]
            unsafe {
                for value in slots.into_iter().flatten() {
                    value.release();
                }
            }
            Err(problem)
        }
    }
}

/// [`matched`]'s walk, which fills **every** slot or answers why it could not.
///
/// § 6's rules, in the order a command line is read: a word beginning `-` is an
/// option and is matched by its whole spelling; a `bool` option is a flag and
/// takes no value; any other word fills the next positional. An argument nobody
/// wrote takes its declared default where the row carries one, is `false` where
/// it is a flag — § 6's rule, which outranks a flag's own declared default —
/// and is a usage error otherwise.
fn fill(row: &Command, words: &[String], slots: &mut [Option<Value>]) -> Result<(), String> {
    let positions: Vec<usize> = (0..row.args.len())
        .filter(|&index| !row.args[index].is_option())
        .collect();
    let mut filled = 0;
    let mut index = 0;
    while let Some(word) = words.get(index) {
        index += 1;
        if word.starts_with('-') && word.len() > 1 {
            let Some(slot) = row
                .args
                .iter()
                .position(|arg| arg.spellings.iter().any(|spelling| spelling == word))
            else {
                return Err(format!("`{}` takes no option `{word}`", row.name));
            };
            if row.args[slot].conv == ArgConv::Flag {
                slots[slot] = Some(Value::bool(true));
                continue;
            }
            let Some(text) = words.get(index) else {
                return Err(format!("`{word}` takes a value and was written last"));
            };
            index += 1;
            slots[slot] = Some(convert(&row.args[slot], text)?);
            continue;
        }
        let Some(&slot) = positions.get(filled) else {
            return Err(format!(
                "`{}` takes {} argument(s), and `{word}` is one more",
                row.name,
                positions.len()
            ));
        };
        filled += 1;
        slots[slot] = Some(convert(&row.args[slot], word)?);
    }
    for (arg, slot) in row.args.iter().zip(slots.iter_mut()) {
        if slot.is_some() {
            continue;
        }
        // § 6's flag is *given by being written*, so the one an author left
        // out is `false` whatever the declaration's own default says. Ahead of
        // the default below, because that rule is the one exception to it.
        if arg.is_option() && arg.conv == ArgConv::Flag {
            *slot = Some(Value::bool(false));
            continue;
        }
        // The declared default, read through the conversion a *written* word
        // takes — so an argument that was defaulted holds the same value as one
        // that arrived, and there is one conversion rather than two.
        if let Some(text) = &arg.default {
            *slot = Some(convert(arg, text)?);
            continue;
        }
        if arg.is_option() {
            return Err(format!(
                "`{}` needs a value and was not written",
                summary_spelling(&arg.spellings)
            ));
        }
        return Err(format!(
            "`{}` takes a <{}> and was given none",
            row.name, arg.param
        ));
    }
    Ok(())
}

/// One word, as the value the parameter behind `arg` receives — § 6's "a
/// matched value's type comes from the parameter", read off the conversion the
/// checker recorded on the row.
fn convert(arg: &CommandArg, text: &str) -> Result<Value, String> {
    match &arg.conv {
        ArgConv::Text => Ok(Value::str(NvsStr::new(text.as_bytes()))),
        // A *positional* `bool`, which has no spelling to be written and so
        // reads the words instead — an option never reaches here.
        ArgConv::Flag => match text {
            "true" => Ok(Value::bool(true)),
            "false" => Ok(Value::bool(false)),
            _ => Err(format!(
                "`{}` is `true` or `false`, and `{text}` is neither",
                arg.param
            )),
        },
        ArgConv::Int => text.parse::<i64>().map(Value::int).map_err(|_| {
            format!(
                "`{}` takes a whole number, and `{text}` is not one",
                arg.param
            )
        }),
        ArgConv::Uint => text.parse::<u64>().map(Value::uint).map_err(|_| {
            format!(
                "`{}` takes a whole number that is not negative, and `{text}` is not one",
                arg.param
            )
        }),
        // Both of these read the runtime's own parse rather than a second one
        // written here, which is what keeps a word a command line supplies and
        // a segment a route matches on one grammar: `nvs_runtime::routes`'
        // `convert` is these same two lines, one table along.
        ArgConv::Decimal => Decimal::parse(text).map(Value::decimal).ok_or_else(|| {
            format!(
                "`{}` takes an exact decimal number, and `{text}` is not one",
                arg.param
            )
        }),
        ArgConv::Uuid => nvs_runtime::uuid::read(text)
            .map(crate::uuid::of_octets)
            .ok_or_else(|| format!("`{}` takes a UUID, and `{text}` is not one", arg.param)),
        // The word itself, and the one conversion this function does not
        // finish: a class's own `parse` needs the context that [`parse_each`]
        // has and a walk over words has not. Nothing typed `string` here
        // reaches a parameter that declared a class — that second pass runs
        // over this same list before the handler is called.
        ArgConv::Parses(_) => Ok(Value::str(NvsStr::new(text.as_bytes()))),
        // § 3's closed set, and the one conversion whose refusal can name every
        // value it would have accepted — a command line is a person typing, so
        // the set is worth more in the message than the type's own spelling.
        // The word itself is the value, which is `nvs_runtime::commands::
        // ArgConv::OneOf`'s own doc to justify.
        ArgConv::OneOf(admitted) => {
            if admitted.iter().any(|value| value == text) {
                Ok(Value::str(NvsStr::new(text.as_bytes())))
            } else {
                Err(format!(
                    "`{}` is one of {}, and `{text}` is none of them",
                    arg.param,
                    admitted
                        .iter()
                        .map(|value| format!("`{value}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            }
        }
        // § 6's other closed set, and the refusal is the same sentence with the
        // enum's own name in it — the words are cases *of* something here, which
        // is the one thing a union of literals has nothing to say. The value is
        // the case's backing integer and there is no case object to build:
        // `nvs_runtime::commands::ArgConv::Enum` is the home of both.
        ArgConv::Enum { class, cases } => cases
            .iter()
            .find(|(case, _)| case == text)
            .map(|(_, value)| match value {
                CaseValue::Int(number) => Value::int(*number),
                CaseValue::Uint(number) => Value::uint(*number),
            })
            .ok_or_else(|| {
                format!(
                    "`{}` is a `{class}` — one of {} — and `{text}` is none of them",
                    arg.param,
                    cases
                        .iter()
                        .map(|(case, _)| format!("`{case}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }),
        // Unreachable: the helper refuses a row carrying one before it reads a
        // word, so that the gap answers as a `LogicError` rather than as a
        // usage error about the argument that happened to arrive.
        ArgConv::Unconverted => Err(format!(
            "`{}` has a type no matcher converts yet",
            arg.param
        )),
    }
}

/// Why a word did not become the value its parameter declared.
///
/// Two answers rather than one because [`parse_each`] reaches a program's own
/// `parse`, and that call can fail in a way no usage page could honestly
/// describe. A word the class refused is [`Self::Usage`] — the command line is
/// input, and § 6 answers input with a page. Anything the *engine* could not do
/// is [`Self::Fault`] and is returned unchanged, so a fatal never reaches a
/// person as advice about the word they typed.
#[derive(Debug)]
enum Refused {
    /// The sentence a usage page leads with.
    Usage(String),
    /// A fault the caller answers with as its own.
    Fault(Fault),
}

/// Replaces every `Parses` argument's word with what that class's own `parse`
/// answered, over the list [`matched`] filled and before the handler is called.
///
/// `nvs_runtime::commands::ArgConv::Parses` is the home of why a command line
/// may take this reach where a route match may not, and of why the member
/// called is `parse` rather than `rule:expressions/try-parse`'s twin. Two
/// things are decided here instead. It is a **second pass** rather than an arm
/// of [`convert`]: that function is a walk over words alone, so keeping the one
/// conversion that needs a context out of it keeps every other one answerable
/// without building one. And a refusal becomes the sentence a usage page leads
/// with, carrying what the class itself said, because a command line is input —
/// `nvs_runtime::commands::ArgConv::OneOf`'s rule, reached from the other side.
///
/// Nothing survives an `Err`, exactly as in [`matched`]: the values this frame
/// was handed are released here, so the caller sweeps only the list it got back.
fn parse_each(
    ctx: &mut nvs_runtime::Ctx,
    row: &Command,
    values: Vec<Value>,
) -> Result<Vec<Value>, Refused> {
    let mut done: Vec<Value> = Vec::with_capacity(values.len());
    let mut rest = values.into_iter();
    for arg in &row.args {
        let Some(word) = rest.next() else {
            break;
        };
        let ArgConv::Parses(class) = &arg.conv else {
            done.push(word);
            continue;
        };
        let outcome = nvs_runtime::call_static(ctx, &format!("{class}::parse"), &[word]);
        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference `word` holds, and \
                      `call_static` retained its own for the callee to release"
        )]
        unsafe {
            word.release();
        }
        let refusal = match outcome {
            Ok(Some(value)) => {
                done.push(value);
                continue;
            }
            // Unreachable from source: `E0746` refuses a parameter whose class
            // does not implement `Parses`, and one that does owes `parse` to
            // `nvs_types::conformance` before a row naming it is ever built. A
            // miss here is a class table that does not match the command table
            // built beside it, which no program can write its way into.
            Ok(None) => Refused::Fault(Fault::fatal(format!(
                "internal error: `{class}::parse` is not in this program's class table"
            ))),
            // Taking the pending message is the catch itself, and the reason
            // this arm exists rather than a `?`.
            Err(Fault::Pending(_)) => Refused::Usage(match ctx.take_pending() {
                Some(said) => format!("`{}` takes a `{class}`, and {said}", arg.param),
                None => format!("`{}` takes a `{class}`, and that one refused", arg.param),
            }),
            Err(fault) => Refused::Fault(fault),
        };
        #[expect(
            unsafe_code,
            reason = "every value in either list is one this frame owns and \
                      nothing else has seen, as on `matched`'s failing path"
        )]
        unsafe {
            for value in done {
                value.release();
            }
            for value in rest {
                value.release();
            }
        }
        return Err(refusal);
    }
    done.extend(rest);
    Ok(done)
}

/// The spelling an option is *summarized* by — its long one where it has one,
/// and otherwise the only one it has. See the module doc for why the usage line
/// names one and the `options:` block names all.
fn summary_spelling(spellings: &[String]) -> &str {
    spellings
        .iter()
        .find(|spelling| spelling.starts_with("--"))
        .or_else(|| spellings.first())
        .map_or("", String::as_str)
}

/// § 6's first line: the command and the shape of a command line that runs it.
fn usage_line(row: &Command) -> String {
    let mut line = format!("usage: {}", row.name);
    for arg in &row.args {
        if arg.is_option() {
            line.push_str(&format!(" [{}]", summary_spelling(&arg.spellings)));
        } else if arg.default.is_some() {
            // Brackets say *optional*, which is what a declared default makes a
            // positional: the command line that leaves it out is matched.
            line.push_str(&format!(" [<{}>]", arg.param));
        } else {
            line.push_str(&format!(" <{}>", arg.param));
        }
    }
    line
}

/// One line of a `heading:` block: the spelling the page lists it under, the
/// declaration behind it, and the author's own `about:`.
///
/// `declared` is the empty string for a line the page has no declaration for,
/// which is what collapses the middle column for a whole block rather than
/// leaving one line short of the others — see the module doc for why it is both
/// halves of a declaration or neither.
struct Entry<'a> {
    left: String,
    declared: String,
    about: Option<&'a str>,
}

/// One `heading:` and its indented, column-aligned entries, or nothing at all
/// for a heading with no entry under it — an empty section reads as a claim
/// that the command has no arguments, which is what the *absence* says.
///
/// Each column is as wide as this block's own widest entry, and a column every
/// entry left empty takes no width at all.
fn block(heading: &str, entries: &[Entry<'_>]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let widest = entries
        .iter()
        .map(|entry| entry.left.len())
        .max()
        .unwrap_or(0);
    let declared_width = entries
        .iter()
        .map(|entry| entry.declared.len())
        .max()
        .unwrap_or(0);
    let mut out = format!("\n{heading}:\n");
    for entry in entries {
        let mut line = format!("  {:<widest$}", entry.left);
        if declared_width > 0 {
            line.push_str(&format!("  {:<declared_width$}", entry.declared));
        }
        if let Some(about) = entry.about {
            line.push_str("  ");
            line.push_str(about);
        }
        // The padding is what aligns the *next* column, so a line whose later
        // columns are empty ends at its last word rather than at the width.
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// What the declaration wrote for `arg`, as a page names it: the parameter's
/// type, with `= <default>` after it where the command line may leave the
/// argument out. The empty string where the page cannot read the type, since it
/// is both halves or neither — the module doc owns that rule and the flag's
/// exception to the second half.
fn declaration_of(arg: &CommandArg, signature: Option<&MethodRow>) -> String {
    let Some(signature) = signature else {
        return String::new();
    };
    let Some(slot) = signature
        .param_names
        .iter()
        .position(|name| *name == arg.param)
    else {
        return String::new();
    };
    let Some(declared) = signature
        .param_types
        .get(slot)
        .filter(|declared| !declared.is_empty())
    else {
        return String::new();
    };
    match &arg.default {
        Some(default) if !(arg.is_option() && arg.conv == ArgConv::Flag) => {
            format!("{declared} = {default}")
        }
        _ => declared.clone(),
    }
}

/// The handler's own row in the class table, which is where a parameter's
/// declared type is and the only place it is.
///
/// [`Command::handler`] is the `Class::method` label the compiler wrote, so the
/// reach is the one [`nvs_runtime::call_static`] already takes to dispatch —
/// `None` for a context holding no such class, which is a page with no
/// declaration column rather than a refusal: `help` answers for a table the
/// compiler built, and a class table it cannot see is not the caller's mistake.
fn signature_of<'a>(ctx: &'a nvs_runtime::Ctx, row: &Command) -> Option<&'a MethodRow> {
    let (class, method) = row.handler.split_once("::")?;
    let desc = ctx.class_desc(class)?;
    #[expect(
        unsafe_code,
        reason = "`class_desc` answers with a pointer into the compiled unit's class table, \
                  which outlives this context and is never rewritten while a member of it is \
                  running"
    )]
    let desc = unsafe { &*desc };
    desc.method_row(method)
}

/// One command's page, over the handler's signature where the caller could
/// reach one — [`signature_of`] is how, and [`declaration_of`] is what the
/// middle column becomes without it.
fn page_for(row: &Command, signature: Option<&MethodRow>) -> String {
    let mut page = format!("{}\n", usage_line(row));
    if let Some(about) = &row.about {
        page.push_str(&format!("\n{about}\n"));
    }
    let positionals: Vec<Entry<'_>> = row
        .args
        .iter()
        .filter(|arg| !arg.is_option())
        .map(|arg| Entry {
            left: format!("<{}>", arg.param),
            declared: declaration_of(arg, signature),
            about: arg.about.as_deref(),
        })
        .collect();
    let options: Vec<Entry<'_>> = row
        .args
        .iter()
        .filter(|arg| arg.is_option())
        .map(|arg| Entry {
            left: arg.spellings.join(", "),
            declared: declaration_of(arg, signature),
            about: arg.about.as_deref(),
        })
        .collect();
    page.push_str(&block("arguments", &positionals));
    page.push_str(&block("options", &options));
    page
}

/// The program's own page: what commands there are, rather than what one of
/// them takes.
fn overview(table: &CommandTable) -> String {
    let mut page = String::from("usage: <command> [arguments]\n");
    if table.rows().is_empty() {
        page.push_str("\nthis program declares no command\n");
        return page;
    }
    // No declaration column: a command is not a parameter, so there is no type
    // to name beside its name.
    let commands: Vec<Entry<'_>> = table
        .rows()
        .iter()
        .map(|row| Entry {
            left: row.name.clone(),
            declared: String::new(),
            about: row.about.as_deref(),
        })
        .collect();
    page.push_str(&block("commands", &commands));
    page
}

// ------------------------------------------------------ the completion scripts

/// [`crate::cli::SHELL`]'s four cases, as a generator is selected.
///
/// The ordinals are written out rather than derived from that roster, for the
/// reason `crate::cli`'s own `depth_ordinal` states: two rosters declared in two
/// modules for two readers, and a cast that looked equivalent would silently
/// answer the wrong case the first time either gained an entry.
#[derive(Clone, Copy)]
enum Shell {
    Bash,
    Zsh,
    Fish,
    Pwsh,
}

/// The [`Shell`] a `Core\Cli\Shell` case arrived as.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is no case of the enum, which is what
/// `crate::cli`'s own `stream_of` answers and for the same reason: it is
/// unreachable from source, since the parameter is `CoreTy::Enum` and `E0401`
/// refuses anything else before this body runs.
fn shell_of(value: &Value) -> Result<Shell, Fault> {
    match value.as_int() {
        Some(0) => Ok(Shell::Bash),
        Some(1) => Ok(Shell::Zsh),
        Some(2) => Ok(Shell::Fish),
        Some(3) => Ok(Shell::Pwsh),
        _ => Err(Fault::fatal(format!(
            "Core\\Command::completions expected a `Core\\Cli\\Shell` case, got tag {} value {:?}",
            value.tag_byte(),
            value.as_int()
        ))),
    }
}

/// A description on one line. Every script here is line-oriented, so a newline
/// inside an `about:` would end the line it is written into and leave the rest
/// of the sentence as shell code.
fn one_line(about: Option<&str>) -> Option<String> {
    about.map(|text| text.replace(['\n', '\r'], " "))
}

/// `text` inside a POSIX shell's single quotes — the one quoting that has no
/// escapes inside it at all, so a `'` is closed, escaped and reopened.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// `text` inside PowerShell's single quotes, where the escape is doubling.
fn pwsh_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// A word inside `compgen -W`'s list, which `bash` splits on whitespace and
/// then **expands** — so a command named `$(id)` reaches that list as the
/// program's own text and leaves it as a command the user's shell runs the
/// moment they press Tab.
///
/// A backslash in front of each character that starts an expansion survives
/// the split and leaves the word as itself. A name carrying a space is not a
/// name any command line can select — `nvs_runtime::commands` matches the
/// first *word* — so the split is not a second thing to escape around.
fn compgen_word(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '$' | '`') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// A `_describe` entry's **value** half, which is everything up to the first
/// colon `zsh` does not see escaped.
///
/// A command named `db:migrate` is an ordinary spelling and a common one, so
/// the name says which colons are its own rather than losing everything past
/// the first — and the backslashes go first, since the escape is what a
/// backslash already means here.
fn zsh_value(name: &str) -> String {
    name.replace('\\', "\\\\").replace(':', "\\:")
}

/// The program's name as a shell **identifier**, for the function a `bash` or
/// `zsh` script defines. Everything that is not an ASCII letter, digit or `_`
/// becomes `_`, and a leading digit gains one: `my-prog` is an entirely
/// ordinary executable name and `_my-prog()` is a syntax error in both shells.
fn identifier(program: &str) -> String {
    let mut name: String = program
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    if name.starts_with(|ch: char| ch.is_ascii_digit()) {
        name.insert(0, '_');
    }
    name
}

/// Every spelling of every option `row` declares, in declaration order — the
/// words that may follow this command's name.
fn spellings(row: &Command) -> Vec<&str> {
    row.args
        .iter()
        .flat_map(|arg| arg.spellings.iter().map(String::as_str))
        .collect()
}

/// A description inside a zsh spec's `[…]`, or nothing for an option that
/// declares none. Only `]` needs the escape: it would close the bracket, and
/// the single quotes around the whole spec take care of the rest.
fn zsh_bracket(about: Option<String>) -> String {
    about.map_or_else(String::new, |text| {
        format!("[{}]", text.replace(']', "\\]"))
    })
}

/// `bash`: one function over `COMP_WORDS`, registered with `complete -F`.
fn bash_script(program: &str, table: &CommandTable) -> String {
    let function = format!("_{}_complete", identifier(program));
    let names: Vec<String> = table
        .rows()
        .iter()
        .map(|row| compgen_word(&row.name))
        .collect();
    let mut out =
        format!("# bash completion for {program} — generated by Core\\Command::completions\n");
    out.push_str(&format!("{function}() {{\n"));
    out.push_str("    local cur=\"${COMP_WORDS[COMP_CWORD]}\"\n");
    out.push_str("    if [ \"$COMP_CWORD\" -eq 1 ]; then\n");
    out.push_str(&format!(
        "        COMPREPLY=($(compgen -W {} -- \"$cur\"))\n",
        quoted(&names.join(" "))
    ));
    out.push_str("        return\n");
    out.push_str("    fi\n");
    out.push_str("    local options=''\n");
    out.push_str("    case \"${COMP_WORDS[1]}\" in\n");
    for row in table.rows() {
        // A command declaring no option gets no arm: `options` is already the
        // empty list, and an arm setting it to one again would say that this
        // command was considered and found to have nothing, which is what
        // falling through says anyway.
        let options = spellings(row);
        if !options.is_empty() {
            out.push_str(&format!(
                "        {}) options={} ;;\n",
                quoted(&row.name),
                quoted(
                    &options
                        .iter()
                        .map(|spelling| compgen_word(spelling))
                        .collect::<Vec<String>>()
                        .join(" ")
                )
            ));
        }
    }
    out.push_str("    esac\n");
    out.push_str("    COMPREPLY=($(compgen -W \"$options\" -- \"$cur\"))\n");
    out.push_str("}\n");
    out.push_str(&format!("complete -F {function} {}\n", quoted(program)));
    out
}

/// `zsh`: a `#compdef` function, `_describe` for the command word and
/// `_arguments` for one command's options.
fn zsh_script(program: &str, table: &CommandTable) -> String {
    let function = format!("_{}", identifier(program));
    let mut out = format!("#compdef {program}\n");
    out.push_str(&format!(
        "# zsh completion for {program} — generated by Core\\Command::completions\n"
    ));
    out.push_str(&format!("{function}() {{\n"));
    out.push_str("    local -a commands\n");
    out.push_str("    commands=(\n");
    for row in table.rows() {
        // `_describe` splits an entry at its first colon, so both halves say
        // which colons are their own: the name escapes its ([`zsh_value`]),
        // and the description is everything past the split and has nothing to
        // escape with, so its colons become spaces. A command that declared no
        // `about:` is the bare name, never a name with an empty description
        // hanging off a colon.
        let name = zsh_value(&row.name);
        let entry = match one_line(row.about.as_deref()) {
            Some(about) => format!("{name}:{}", about.replace(':', " ")),
            None => name,
        };
        out.push_str(&format!("        {}\n", quoted(&entry)));
    }
    out.push_str("    )\n");
    out.push_str("    if (( CURRENT == 2 )); then\n");
    out.push_str("        _describe 'command' commands\n");
    out.push_str("        return\n");
    out.push_str("    fi\n");
    out.push_str("    case \"${words[2]}\" in\n");
    for row in table.rows() {
        let specs: Vec<String> = row
            .args
            .iter()
            .filter(|arg| arg.is_option())
            .flat_map(|arg| {
                let about = zsh_bracket(one_line(arg.about.as_deref()));
                arg.spellings
                    .iter()
                    .map(move |spelling| quoted(&format!("{spelling}{about}")))
            })
            .collect();
        // A command declaring no option gets no arm at all: `_arguments` with
        // nothing after it is a call with no specification, not a completion of
        // nothing.
        if !specs.is_empty() {
            out.push_str(&format!(
                "        {}) _arguments {} ;;\n",
                quoted(&row.name),
                specs.join(" ")
            ));
        }
    }
    out.push_str("    esac\n");
    out.push_str("}\n");
    out.push_str(&format!("{function} \"$@\"\n"));
    out
}

/// `fish`: one `complete -c` line per command and per option spelling, which is
/// the whole of that shell's completion language.
fn fish_script(program: &str, table: &CommandTable) -> String {
    let name = quoted(program);
    let mut out =
        format!("# fish completion for {program} — generated by Core\\Command::completions\n");
    for row in table.rows() {
        out.push_str(&format!(
            "complete -c {name} -n '__fish_use_subcommand' -a {}",
            quoted(&row.name)
        ));
        if let Some(about) = one_line(row.about.as_deref()) {
            out.push_str(&format!(" -d {}", quoted(&about)));
        }
        out.push('\n');
        let seen = quoted(&format!("__fish_seen_subcommand_from {}", row.name));
        for arg in row.args.iter().filter(|arg| arg.is_option()) {
            let about = one_line(arg.about.as_deref());
            for spelling in &arg.spellings {
                let (flag, bare) = fish_flag(spelling);
                out.push_str(&format!(
                    "complete -c {name} -n {seen} {flag} {}",
                    quoted(bare)
                ));
                if let Some(about) = &about {
                    out.push_str(&format!(" -d {}", quoted(about)));
                }
                out.push('\n');
            }
        }
    }
    out
}

/// A spelling as fish names it: that shell writes the dashes itself, taking
/// `-l long` and `-s s` rather than the spelling a command line carries.
fn fish_flag(spelling: &str) -> (&'static str, &str) {
    spelling.strip_prefix("--").map_or_else(
        || ("-s", spelling.trim_start_matches('-')),
        |long| ("-l", long),
    )
}

/// PowerShell: one `Register-ArgumentCompleter -Native` script block, which is
/// how that shell completes a program it did not define as a cmdlet.
fn pwsh_script(program: &str, table: &CommandTable) -> String {
    let names: Vec<String> = table
        .rows()
        .iter()
        .map(|row| pwsh_quoted(&row.name))
        .collect();
    let mut out = format!(
        "# PowerShell completion for {program} — generated by Core\\Command::completions\n"
    );
    out.push_str(&format!(
        "Register-ArgumentCompleter -Native -CommandName {} -ScriptBlock {{\n",
        pwsh_quoted(program)
    ));
    out.push_str("    param($wordToComplete, $commandAst, $cursorPosition)\n");
    out.push_str(
        "    $words = @($commandAst.CommandElements | Select-Object -Skip 1 | ForEach-Object { $_.ToString() })\n",
    );
    // The word being typed is itself an element of the AST, so the command is
    // only selected once some *other* word precedes it.
    out.push_str(
        "    $command = if ($words.Count -gt 0 -and $words[0] -ne $wordToComplete) { $words[0] } else { '' }\n",
    );
    out.push_str("    $candidates = @()\n");
    out.push_str("    if ($command -eq '') {\n");
    out.push_str(&format!("        $candidates = @({})\n", names.join(", ")));
    out.push_str("    } else {\n");
    out.push_str("        switch ($command) {\n");
    for row in table.rows() {
        // No arm for a command with no option, for `bash_script`'s reason.
        let options: Vec<String> = spellings(row)
            .iter()
            .map(|spelling| pwsh_quoted(spelling))
            .collect();
        if !options.is_empty() {
            out.push_str(&format!(
                "            {} {{ $candidates = @({}) }}\n",
                pwsh_quoted(&row.name),
                options.join(", ")
            ));
        }
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str(
        "    $candidates | Where-Object { $_ -like \"$wordToComplete*\" } | ForEach-Object {\n",
    );
    out.push_str(
        "        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)\n",
    );
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::commands::CommandArg;
    use nvs_runtime::{ClassTable, Ctx, ErrorClass, MethodRow, NvsFn, OK, OutputSink, call};

    /// Releases what [`matched`] handed back, which is what the helper does
    /// after the call it made them for.
    fn release(values: Vec<Value>) {
        #[expect(
            unsafe_code,
            reason = "these are the references `matched` made and handed to \
                      this frame, and no callee has seen them"
        )]
        unsafe {
            for value in values {
                value.release();
            }
        }
    }

    /// `rule:tooling/commands-are-compiled`'s own example, as a row.
    fn deploy() -> Command {
        Command {
            name: "deploy".to_owned(),
            about: Some("Push the current build".to_owned()),
            handler: "Deployer::deploy".to_owned(),
            args: vec![
                CommandArg {
                    param: "target".to_owned(),
                    spellings: Vec::new(),
                    about: None,
                    conv: ArgConv::Text,
                    default: None,
                },
                CommandArg {
                    param: "dryRun".to_owned(),
                    spellings: vec!["-n".to_owned(), "--dryRun".to_owned()],
                    about: Some("Print what would happen".to_owned()),
                    conv: ArgConv::Flag,
                    default: None,
                },
            ],
        }
    }

    /// The usage line reads a positional and an option apart by the one field
    /// that distinguishes them, and summarizes a two-spelling option by its
    /// long form — the module doc's rule, which is otherwise only visible in a
    /// rendered page.
    #[test]
    fn the_usage_line_writes_positionals_bare_and_options_bracketed() {
        assert_eq!(
            usage_line(&deploy()),
            "usage: deploy <target> [--dryRun]",
            "a positional is `<name>` and an option is its long spelling in brackets"
        );
    }

    /// Both blocks appear, aligned, and the option's own `about:` rides with
    /// every spelling it answers to rather than with the summarized one.
    ///
    /// No signature, so no declaration column — the half of the module doc's
    /// "both or neither" a caller holding no context lands on.
    #[test]
    fn a_page_carries_both_blocks_and_every_spelling() {
        assert_eq!(
            page_for(&deploy(), None),
            "usage: deploy <target> [--dryRun]\n\
             \n\
             Push the current build\n\
             \n\
             arguments:\n\
             \x20 <target>\n\
             \n\
             options:\n\
             \x20 -n, --dryRun  Print what would happen\n"
        );
    }

    /// A command with no argument at all prints neither heading: an empty
    /// `arguments:` block is a claim, and the absence is the true one.
    #[test]
    fn a_command_with_no_argument_prints_no_heading() {
        let row = Command {
            name: "version".to_owned(),
            about: None,
            handler: "App::version".to_owned(),
            args: Vec::new(),
        };
        assert_eq!(page_for(&row, None), "usage: version\n");
    }

    /// The declaration column is read off the handler's own row, so a page
    /// names the type the *source* wrote rather than the conversion the table
    /// carries — `uint` and `int` pick one `ArgConv` between them, and a page
    /// derived from that arm could not tell them apart.
    #[test]
    fn a_page_names_the_type_each_parameter_was_declared_at() {
        let mut declared = deploy_row();
        declared.param_names = vec!["target".to_owned(), "dryRun".to_owned()];
        declared.param_types = vec!["string".to_owned(), "bool".to_owned()];
        let ctx = dispatching(&["deploy"], vec![declared]);
        let row = deploy();
        assert_eq!(
            page_for(&row, signature_of(&ctx, &row)),
            "usage: deploy <target> [--dryRun]\n\
             \n\
             Push the current build\n\
             \n\
             arguments:\n\
             \x20 <target>  string\n\
             \n\
             options:\n\
             \x20 -n, --dryRun  bool  Print what would happen\n"
        );
    }

    /// A declared default rides with the type it was declared beside, and a
    /// flag's is withheld: § 6 gives an option declared `bool` by its being
    /// written, so `= true` on that line would name a default the matcher does
    /// not honour — the same divergence
    /// [`an_unwritten_argument_takes_its_declared_default`] asserts on the
    /// values.
    ///
    /// A parameter the handler's row does not name keeps its line and loses
    /// its declaration, which is what makes the column a page's answer about
    /// each parameter rather than about the command.
    #[test]
    fn a_default_rides_with_its_type_and_a_flag_names_neither() {
        let row = Command {
            name: "fetch".to_owned(),
            about: None,
            handler: "Deployer::fetch".to_owned(),
            args: vec![
                CommandArg {
                    param: "url".to_owned(),
                    spellings: Vec::new(),
                    about: None,
                    conv: ArgConv::Text,
                    default: Some("https://example.test".to_owned()),
                },
                CommandArg {
                    param: "retries".to_owned(),
                    spellings: vec!["--retries".to_owned()],
                    about: None,
                    conv: ArgConv::Uint,
                    default: Some("3".to_owned()),
                },
                CommandArg {
                    param: "loud".to_owned(),
                    spellings: vec!["--loud".to_owned()],
                    about: None,
                    conv: ArgConv::Flag,
                    default: Some("true".to_owned()),
                },
                CommandArg {
                    param: "quiet".to_owned(),
                    spellings: vec!["--quiet".to_owned()],
                    about: None,
                    conv: ArgConv::Flag,
                    default: None,
                },
            ],
        };
        let mut declared = deploy_row();
        declared.name = "fetch".to_owned();
        declared.param_names = vec!["url".to_owned(), "retries".to_owned(), "loud".to_owned()];
        declared.param_types = vec!["string".to_owned(), "uint".to_owned(), "bool".to_owned()];
        let ctx = dispatching(&["fetch"], vec![declared]);
        assert_eq!(
            page_for(&row, signature_of(&ctx, &row)),
            "usage: fetch [<url>] [--retries] [--loud] [--quiet]\n\
             \n\
             arguments:\n\
             \x20 <url>  string = https://example.test\n\
             \n\
             options:\n\
             \x20 --retries  uint = 3\n\
             \x20 --loud     bool\n\
             \x20 --quiet\n"
        );
    }

    /// A context that was handed no table and a program that declares no
    /// command are one case — `nvs_runtime::commands` owns why — so the page
    /// says so in a sentence rather than printing an empty `commands:` heading.
    #[test]
    fn a_program_with_no_command_says_so_rather_than_printing_a_heading() {
        let page = overview(&CommandTable::default());
        assert_eq!(
            page,
            "usage: <command> [arguments]\n\nthis program declares no command\n"
        );
        assert!(!page.contains("commands:"));
    }

    /// The overview lists every row in the table's own order, which is the
    /// compiler's load order — a page that sorted them would disagree with
    /// `Core\Command::run`'s own dispatch order for no reader's benefit.
    #[test]
    fn the_overview_lists_every_command_in_table_order() {
        let table = CommandTable::new(vec![
            deploy(),
            Command {
                name: "build".to_owned(),
                about: None,
                handler: "Builder::build".to_owned(),
                args: Vec::new(),
            },
        ]);
        assert_eq!(
            overview(&table),
            "usage: <command> [arguments]\n\
             \n\
             commands:\n\
             \x20 deploy  Push the current build\n\
             \x20 build\n"
        );
    }

    /// `rule:tooling/commands-are-compiled`'s flag is *given by being written*, so the answer for an
    /// option nobody wrote is `false` rather than a missing slot — and both of
    /// its spellings write the same `true`, which is the property a matcher
    /// keyed on one of them would still pass a single-spelling test with.
    #[test]
    fn a_flag_is_false_unless_it_is_written_and_every_spelling_writes_it() {
        let row = deploy();
        for (words, want) in [
            (vec!["prod".to_owned()], false),
            (vec!["prod".to_owned(), "-n".to_owned()], true),
            (vec!["prod".to_owned(), "--dryRun".to_owned()], true),
        ] {
            let values = matched(&row, &words).expect("a matched command line");
            assert_eq!(values.len(), 2, "one value per declared parameter");
            assert_eq!(values[0].as_text(), Some("prod"), "the positional");
            assert_eq!(values[1].as_bool(), Some(want), "{words:?}");
            release(values);
        }
    }

    /// The two boundaries of the positional list, named together: the last word
    /// it accepts and the first it refuses, since a matcher that stops one
    /// entry early answers plausibly against either half alone.
    #[test]
    fn a_positional_list_is_full_at_its_declared_count() {
        let row = deploy();
        let filled = matched(&row, &["prod".to_owned()]).expect("one positional is the whole list");
        release(filled);
        assert_eq!(
            matched(&row, &[]).unwrap_err(),
            "`deploy` takes a <target> and was given none"
        );
        assert_eq!(
            matched(&row, &["prod".to_owned(), "staging".to_owned()]).unwrap_err(),
            "`deploy` takes 1 argument(s), and `staging` is one more"
        );
    }

    /// A `uint` option converts during matching (§ 6), and the three ways a
    /// command line can fail to give it one are usage errors rather than
    /// crashes — including leaving it out, which is a usage error exactly
    /// because this row declared no default for it.
    #[test]
    fn a_uint_option_converts_and_refuses_what_is_not_a_number() {
        let row = Command {
            name: "fetch".to_owned(),
            about: None,
            handler: "Fetcher::fetch".to_owned(),
            args: vec![CommandArg {
                param: "retries".to_owned(),
                spellings: vec!["--retries".to_owned()],
                about: None,
                conv: ArgConv::Uint,
                default: None,
            }],
        };
        let values = matched(&row, &["--retries".to_owned(), "3".to_owned()])
            .expect("a written option carries its value");
        assert_eq!(values[0].as_uint(), Some(3));
        release(values);
        assert_eq!(
            matched(&row, &["--retries".to_owned(), "-1".to_owned()]).unwrap_err(),
            "`retries` takes a whole number that is not negative, and `-1` is not one"
        );
        assert_eq!(
            matched(&row, &["--retries".to_owned()]).unwrap_err(),
            "`--retries` takes a value and was written last"
        );
        assert_eq!(
            matched(&row, &[]).unwrap_err(),
            "`--retries` needs a value and was not written",
            "an argument with no declared default is one the command line owes"
        );
    }

    /// § 6's `decimal`, on the side a command line fills: a written word
    /// becomes `rule:types/decimal`'s exact number, so nothing declared
    /// `decimal` reaches a handler as the text somebody typed, and a word that
    /// is not one is a usage error rather than a throw.
    ///
    /// The grammar is `nvs_runtime::decimal`'s and never a second one written
    /// here, which is what the exponent asserts: a hand-written digit check
    /// passes every other line of this test and fails that one.
    #[test]
    fn a_decimal_command_argument_is_converted_and_no_longer_unconverted() {
        let row = Command {
            name: "charge".to_owned(),
            about: None,
            handler: "Billing::charge".to_owned(),
            args: vec![CommandArg {
                param: "amount".to_owned(),
                spellings: vec![],
                about: None,
                conv: ArgConv::Decimal,
                default: None,
            }],
        };
        // The value and its scale, not the digits: `19.90` is not `19.9`, and
        // an integral word is a decimal too.
        let values = matched(&row, &["19.90".to_owned()]).expect("a decimal word converts");
        assert_eq!(values[0].as_decimal(), Decimal::parse("19.90"));
        assert_eq!(values[0].as_text(), None, "the word does not survive");
        release(values);
        let integral = matched(&row, &["7".to_owned()]).expect("a decimal word converts");
        assert_eq!(integral[0].as_decimal(), Decimal::parse("7"));
        release(integral);
        let exponent = matched(&row, &["1e3".to_owned()]).expect("a decimal word converts");
        assert_eq!(exponent[0].as_decimal(), Decimal::parse("1e3"));
        release(exponent);

        assert_eq!(
            matched(&row, &["19.90usd".to_owned()]).unwrap_err(),
            "`amount` takes an exact decimal number, and `19.90usd` is not one"
        );
    }

    /// § 3's union of literal types, as the closed set a word is narrowed to:
    /// every member converts to its own word, and anything else is a **usage**
    /// error naming every value that would have been accepted — a command line
    /// is input, so a word outside the set is never a throw.
    ///
    /// Asserted over the whole set rather than over one member, because a
    /// matcher that admitted the first entry and stopped answers the first line
    /// correctly. The refused word is the half that says this is a narrowing at
    /// all: `ArgConv::Text`, which is what this parameter's type answered
    /// before the set crossed, accepts it.
    #[test]
    fn a_union_of_literal_types_admits_its_own_words_and_refuses_every_other() {
        let row = Command {
            name: "report".to_owned(),
            about: None,
            handler: "Reports::run".to_owned(),
            args: vec![CommandArg {
                param: "format".to_owned(),
                spellings: Vec::new(),
                about: None,
                conv: ArgConv::OneOf(vec!["json".to_owned(), "table".to_owned()]),
                default: None,
            }],
        };
        for admitted in ["json", "table"] {
            let values = matched(&row, &[admitted.to_owned()]).expect("a word the union declares");
            assert_eq!(values[0].as_text(), Some(admitted));
            release(values);
        }
        assert_eq!(
            matched(&row, &["csv".to_owned()]).unwrap_err(),
            "`format` is one of `json`, `table`, and `csv` is none of them"
        );
    }

    /// § 6's own `#[Option] uint $retries = 3`: an argument the command line
    /// left out takes its declared default, through the same conversion a
    /// written word takes — and the flag beside it is `false` by § 6's rule
    /// rather than by the `= true` its declaration wrote, which is the one
    /// place the two answers differ.
    #[test]
    fn an_unwritten_argument_takes_its_declared_default() {
        let row = Command {
            name: "fetch".to_owned(),
            about: None,
            handler: "Fetcher::fetch".to_owned(),
            args: vec![
                CommandArg {
                    param: "url".to_owned(),
                    spellings: Vec::new(),
                    about: None,
                    conv: ArgConv::Text,
                    default: Some("https://example.test".to_owned()),
                },
                CommandArg {
                    param: "retries".to_owned(),
                    spellings: vec!["--retries".to_owned()],
                    about: None,
                    conv: ArgConv::Uint,
                    default: Some("3".to_owned()),
                },
                CommandArg {
                    param: "loud".to_owned(),
                    spellings: vec!["--loud".to_owned()],
                    about: None,
                    conv: ArgConv::Flag,
                    default: Some("true".to_owned()),
                },
            ],
        };

        let values = matched(&row, &[]).expect("every argument answers for itself");
        assert_eq!(values[0].as_text(), Some("https://example.test"));
        assert_eq!(values[1].as_uint(), Some(3));
        assert_eq!(
            values[2].as_bool(),
            Some(false),
            "a flag is given by being written, whatever its declaration defaults to"
        );
        release(values);

        // A written word still wins over the default, which is the half a
        // matcher reading the row in the wrong order would fail.
        let values = matched(
            &row,
            &[
                "https://other.test".to_owned(),
                "--retries".to_owned(),
                "9".to_owned(),
            ],
        )
        .expect("a written option over a declared default");
        assert_eq!(values[0].as_text(), Some("https://other.test"));
        assert_eq!(values[1].as_uint(), Some(9));
        release(values);

        // The usage line says so: a positional carrying a default is bracketed,
        // because the command line that leaves it out is matched.
        assert_eq!(
            usage_line(&row),
            "usage: fetch [<url>] [--retries] [--loud]"
        );
    }

    /// An option written between two positionals does not consume one of their
    /// places: the value order a handler's frame reads is the *declaration*
    /// order, and the command line's own order is only how the words arrive.
    #[test]
    fn an_option_between_positionals_leaves_their_order_alone() {
        let row = Command {
            name: "copy".to_owned(),
            about: None,
            handler: "Files::copy".to_owned(),
            args: vec![
                CommandArg {
                    param: "from".to_owned(),
                    spellings: Vec::new(),
                    about: None,
                    conv: ArgConv::Text,
                    default: None,
                },
                CommandArg {
                    param: "force".to_owned(),
                    spellings: vec!["-f".to_owned()],
                    about: None,
                    conv: ArgConv::Flag,
                    default: None,
                },
                CommandArg {
                    param: "to".to_owned(),
                    spellings: Vec::new(),
                    about: None,
                    conv: ArgConv::Text,
                    default: None,
                },
            ],
        };
        let values = matched(
            &row,
            &["a.txt".to_owned(), "-f".to_owned(), "b.txt".to_owned()],
        )
        .expect("a matched command line");
        assert_eq!(values[0].as_text(), Some("a.txt"));
        assert_eq!(values[1].as_bool(), Some(true));
        assert_eq!(values[2].as_text(), Some("b.txt"));
        release(values);
    }

    thread_local! {
        /// What [`deploy_handler`] was handed, so that a dispatch which reached
        /// the wrong row — or filled the slots in the command line's order
        /// rather than the declaration's — fails here instead of passing on its
        /// status alone.
        static RECEIVED: std::cell::RefCell<Option<(String, bool)>> =
            const { std::cell::RefCell::new(None) };
    }

    /// The status [`deploy_handler`] answers with: neither `0` nor
    /// [`USAGE_STATUS`], so "the handler ran and its own number came back" is
    /// distinguishable from both of `run`'s other exits.
    const HANDLER_STATUS: u64 = 7;

    /// `Deployer::deploy`'s body, as `nvs-codegen` would have compiled it: slot
    /// 0 is the called class, slots 1 and 2 are the two parameters
    /// [`deploy`]'s row declares, and the exit sweep releases every one of them
    /// because `nvs_runtime::call_static` retained them on the way in.
    ///
    /// It records what arrived rather than computing anything from it — the
    /// question is which values reached which slot, and a handler that derived
    /// its answer would hide a swap.
    #[expect(
        unsafe_code,
        reason = "compiled code's own signature, which `call_at` calls through: \
                  exactly three live values and the address of a live `Value` \
                  for the result, neither expressible in the type"
    )]
    unsafe extern "C" fn deploy_handler(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        let target = unsafe { *args.add(1) };
        let dry_run = unsafe { *args.add(2) };
        RECEIVED.with_borrow_mut(|received| {
            *received = Some((
                target.as_text().unwrap_or_default().to_owned(),
                dry_run.as_bool().unwrap_or_default(),
            ));
        });
        unsafe {
            for index in 0..3 {
                (*args.add(index)).release();
            }
            *out = Value::uint(HANDLER_STATUS);
        }
        OK
    }

    /// [`deploy_handler`] as the class table's own row — arity 2, the receiver
    /// excluded, which is what `nvs_types::layout::ClassLayout::methods` writes
    /// for a `static` method with a body.
    fn deploy_row() -> MethodRow {
        MethodRow {
            name: "deploy".to_owned(),
            code: (deploy_handler as NvsFn) as *const u8,
            arity: 2,
            param_tags: 0,
            param_names: Vec::new(),
            param_types: Vec::new(),
            public: true,
            protected: false,
            native: false,
        }
    }

    thread_local! {
        /// The word [`parse_handler`] was handed, so that a pass which answered
        /// plausibly without ever reaching the class — or reached it with the
        /// wrong slot — fails here rather than on the value alone.
        static PARSED: std::cell::RefCell<Option<String>> =
            const { std::cell::RefCell::new(None) };
    }

    /// A `Parses` implementor's `parse`, as `nvs-codegen` would have compiled
    /// it: slot 0 is the called class and slot 1 is the one `string` the
    /// interface declares.
    ///
    /// It answers `true` — a value no argument's own text could be — so that
    /// "the class's answer replaced the word" is asserted on the slot and not
    /// only on what arrived.
    #[expect(
        unsafe_code,
        reason = "compiled code's own signature, which `call_at` calls through: \
                  exactly two live values and the address of a live `Value` for \
                  the result, neither expressible in the type"
    )]
    unsafe extern "C" fn parse_handler(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        let word = unsafe { *args.add(1) };
        PARSED.with_borrow_mut(|parsed| {
            *parsed = word.as_text().map(str::to_owned);
        });
        unsafe {
            for index in 0..2 {
                (*args.add(index)).release();
            }
            *out = Value::bool(true);
        }
        OK
    }

    /// [`parse_handler`]'s twin that refuses, which is the whole of what a
    /// `Parses` implementor does to reject text: it throws, and the message is
    /// the class's own.
    #[expect(
        unsafe_code,
        reason = "as `parse_handler`, and the pending message is set through the \
                  context the ABI hands every compiled function"
    )]
    unsafe extern "C" fn refusing_parse(
        ctx: *mut Ctx,
        args: *const Value,
        _out: *mut Value,
    ) -> i32 {
        unsafe {
            for index in 0..2 {
                (*args.add(index)).release();
            }
            (*ctx).set_pending("no target is spelled `prod`");
        }
        nvs_runtime::THROWN
    }

    /// `code`'s row under the name the interface requires, arity 1 with the
    /// receiver excluded — what `nvs_types::layout::ClassLayout::methods`
    /// writes for a `public static function parse(string): static`.
    fn parse_row(code: NvsFn) -> MethodRow {
        MethodRow {
            name: "parse".to_owned(),
            code: code as *const u8,
            arity: 1,
            param_tags: 0,
            param_names: Vec::new(),
            param_types: Vec::new(),
            public: true,
            protected: false,
            native: false,
        }
    }

    /// [`deploy`]'s positional, declared as a class implementing `Parses`
    /// instead of as `string`, with the word the matcher would have filled it
    /// with.
    fn parses_row() -> (Command, Vec<Value>) {
        let mut row = deploy();
        row.args[0].conv = ArgConv::Parses("Deployer".to_owned());
        let values = vec![
            Value::str(nvs_runtime::NvsStr::new("prod".as_bytes())),
            Value::bool(false),
        ];
        (row, values)
    }

    /// The reach `nvs_runtime::commands::ArgConv::Parses` settles, end to end
    /// on the side that may take it: the word crosses to the class's own
    /// `parse` and what comes back is what the parameter receives.
    #[test]
    fn a_parses_argument_becomes_what_the_classs_own_parse_answered() {
        let mut ctx = dispatching(&["deploy", "prod"], vec![parse_row(parse_handler)]);
        let (row, values) = parses_row();
        PARSED.with_borrow_mut(|parsed| *parsed = None);
        let values = parse_each(&mut ctx, &row, values).expect("the class parsed the word");
        assert_eq!(
            PARSED.with_borrow(Clone::clone),
            Some("prod".to_owned()),
            "the argument's own text reaches `parse`"
        );
        assert_eq!(
            values[0].as_text(),
            None,
            "the word does not survive the pass"
        );
        assert_eq!(values[0].as_bool(), Some(true), "`parse`'s answer does");
        assert_eq!(
            values[1].as_bool(),
            Some(false),
            "an argument no class parses is left where it was"
        );
        release(values);
    }

    /// A class that refuses text is a *usage* error carrying the sentence the
    /// class wrote, never a throw — a command line is input, so the person who
    /// typed the word reads why it was refused.
    #[test]
    fn a_parse_that_refuses_is_a_usage_sentence_carrying_what_the_class_said() {
        let mut ctx = dispatching(&["deploy", "prod"], vec![parse_row(refusing_parse)]);
        let (row, values) = parses_row();
        match parse_each(&mut ctx, &row, values) {
            Err(Refused::Usage(said)) => assert!(
                said.contains("no target is spelled `prod`"),
                "the class's own sentence reaches the page: {said}"
            ),
            Err(Refused::Fault(_)) => panic!("a refusal is not a fault"),
            Ok(_) => panic!("a `parse` that threw converted nothing"),
        }
    }

    /// A context set up the way `nvs run` sets one up before a program reaches
    /// this module: [`deploy`]'s row as the compiled command table, `line` as
    /// the process's argument vector, and a class table carrying whatever
    /// `declares` holds under `Deployer`.
    ///
    /// The class table arrives through `Ctx::set_runtime_error_class` because
    /// that handle *is* this context's anchor into the compiled unit's classes:
    /// `Ctx::class_desc`, which is how `nvs_runtime::call_static` turns the
    /// row's `Class::method` label into an address, reads the table through it
    /// and there is no second registration to make.
    fn dispatching(line: &[&str], declares: Vec<MethodRow>) -> Ctx {
        let mut classes = ClassTable::new();
        let id = classes.define("Deployer", &[] as &[&str], &[]);
        classes.set_methods(id, declares);
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(classes), id));
        ctx.set_commands(std::sync::Arc::new(CommandTable::new(vec![deploy()])));
        ctx.set_command_line(line.iter().map(|word| (*word).to_owned()).collect());
        ctx
    }

    /// `rule:tooling/commands-are-compiled`'s dispatch, end to end: the first word selects a row, the
    /// words past it fill that row's parameters in **declaration** order, and
    /// the handler is reached by the label the compiler wrote into the row —
    /// the whole route `nvs_runtime::call_static` owns, over a class table
    /// built here rather than by a compiler.
    ///
    /// The command line is written `deploy prod -n` and the assertion is that
    /// the option arrived in slot 1: a matcher that handed the handler its
    /// words in the order they were typed would answer the same status.
    // covers: Core\Command::run
    #[test]
    fn command_run_dispatches_through_the_compiled_table() {
        RECEIVED.with_borrow_mut(|received| *received = None);
        let mut ctx = dispatching(&["deploy", "prod", "-n"], vec![deploy_row()]);
        let status = call(super::nvs_core_command_run, &mut ctx, &[])
            .expect("a command line this table answers")
            .as_uint();
        assert_eq!(
            RECEIVED.with_borrow(Clone::clone),
            Some(("prod".to_owned(), true)),
            "the handler ran, and its slots are the row's parameters in declaration order"
        );
        assert_eq!(
            status,
            Some(HANDLER_STATUS),
            "`run` answers with the handler's own status"
        );
    }

    /// A row whose handler the program does not declare is a `LogicError`
    /// rather than a usage page: the table is fixed at compile time, so no
    /// command line can produce this and the author is the one who has to hear
    /// about it.
    #[test]
    fn a_handler_the_program_does_not_declare_is_a_logic_error() {
        let mut ctx = dispatching(&["deploy", "prod"], Vec::new());
        assert_eq!(
            call(super::nvs_core_command_run, &mut ctx, &[])
                .expect_err("a label naming no declared method"),
            nvs_runtime::THROWN
        );
        assert_eq!(
            ctx.take_pending().as_deref(),
            Some(
                "`Deployer::deploy` is the handler `deploy` was declared on, and this program \
                 declares no such method"
            ),
            "the sentence names the label, so an author can grep for it"
        );
    }

    /// The member itself, rather than the [`page_for`] and [`overview`] the
    /// tests above render directly: a page comes back as a `Core\Cli\Text`
    /// whose carrier slot holds the bytes `echo` writes, and a name the table
    /// holds no row for is a `LogicError` rather than an empty page.
    ///
    /// The refusal is the half no `.nvst` case can pin from the outside
    /// alone — the sentence names the word it was given, so an author who
    /// mistyped a command can grep for it.
    // covers: Core\Command::help
    #[test]
    fn help_renders_the_programs_page_and_refuses_a_name_no_command_has() {
        let mut ctx = dispatching(&[], vec![deploy_row()]);
        let page = call(super::nvs_core_command_help, &mut ctx, &[Value::null()])
            .expect("the program's own page");
        let carrier = page.obj_ptr().expect("a `Cli\\Text` is an object");
        let want = overview(&CommandTable::new(vec![deploy()]));
        assert_eq!(
            crate::instance::slot(carrier, nvs_runtime::CARRIER_TEXT_SLOT).as_text(),
            Some(want.as_str()),
            "the carrier slot is the page `echo` writes"
        );
        release(vec![page]);

        let name = Value::str(nvs_runtime::NvsStr::new("deplyo".as_bytes()));
        assert_eq!(
            call(super::nvs_core_command_help, &mut ctx, &[name])
                .expect_err("a name this table holds no row for"),
            nvs_runtime::THROWN
        );
        release(vec![name]);
        assert_eq!(
            ctx.take_pending().as_deref(),
            Some(
                "`Core\\Command::help` was asked for `deplyo`, which this program declares no \
                 `#[Command]` for"
            ),
            "the sentence names the word it was given"
        );
    }

    /// The name this program's scripts register against, in the tests below.
    const PROGRAM: &str = "deployer";

    /// A second row, so that the agreement below is over a table with more than
    /// one command in it — and one declaring neither an option nor an `about:`,
    /// which is the row every generator has a shortcut for.
    fn version() -> Command {
        Command {
            name: "version".to_owned(),
            about: None,
            handler: "Deployer::version".to_owned(),
            args: Vec::new(),
        }
    }

    /// `rule:tooling/commands-are-compiled`'s two generated artefacts read the same table, asserted as
    /// an **agreement** rather than as four expected scripts: every command the
    /// program's own page lists, and every spelling one command's page names,
    /// is named by all four completion scripts as well.
    ///
    /// The four `.nvst` cases pin what each script *looks* like. What this pins
    /// is that none of them grew a second view of the table — a generator
    /// skipping the option that declared no `about:`, or listing only the
    /// commands it happened to have a description for, renders plausibly on its
    /// own and fails here. `fish` is the one shell that does not carry a
    /// spelling through, since it writes the dashes itself, so a script may
    /// name either the spelling or the bare word quoted.
    #[test]
    fn help_and_completions_are_generated_from_the_same_table() {
        let table = CommandTable::new(vec![deploy(), version()]);
        let scripts = [
            bash_script(PROGRAM, &table),
            zsh_script(PROGRAM, &table),
            fish_script(PROGRAM, &table),
            pwsh_script(PROGRAM, &table),
        ];
        let page = overview(&table);
        for row in table.rows() {
            assert!(
                page.contains(&row.name),
                "the program's page lists `{}`",
                row.name
            );
            assert_eq!(
                scripts
                    .iter()
                    .filter(|script| script.contains(&row.name))
                    .count(),
                scripts.len(),
                "every shell's script names the command `{}` the page lists",
                row.name
            );
            let command_page = page_for(row, None);
            for spelling in spellings(row) {
                assert!(
                    command_page.contains(spelling),
                    "`{}`'s page names `{spelling}`",
                    row.name
                );
                let bare = quoted(spelling.trim_start_matches('-'));
                assert_eq!(
                    scripts
                        .iter()
                        .filter(|script| script.contains(spelling) || script.contains(&bare))
                        .count(),
                    scripts.len(),
                    "every shell's script offers `{spelling}` under `{}`",
                    row.name
                );
            }
        }
    }

    /// A command name is the program's own text, and two of the four shells
    /// read an entry apart before they show it: `bash` expands every word of a
    /// `compgen -W` list, and `zsh` ends a `_describe` entry at its first
    /// colon. Both are asserted over one table, because a generator that
    /// escaped for one of them renders plausibly for the other.
    #[test]
    fn a_name_two_shells_would_read_apart_is_written_as_its_own_text() {
        let sneaky = Command {
            name: "db:list$(id)".to_owned(),
            about: None,
            handler: "Deployer::list".to_owned(),
            args: Vec::new(),
        };
        let table = CommandTable::new(vec![sneaky]);
        assert!(
            bash_script(PROGRAM, &table).contains(r"'db:list\$(id)'"),
            "`bash` expands its own list, so the `$` carries a backslash"
        );
        assert!(
            zsh_script(PROGRAM, &table).contains(r"'db\:list$(id)'"),
            "`zsh` ends the command word at a colon, so the colon carries one"
        );
    }

    /// A context with no program name cannot produce a script at all, and says
    /// so rather than inventing a name: the script's whole job is to register
    /// against one, and a served request has none.
    // covers: Core\Command::completions
    #[test]
    fn completions_refuses_a_context_with_no_program_name() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_commands(std::sync::Arc::new(CommandTable::new(vec![deploy()])));
        assert_eq!(
            call(
                super::nvs_core_command_completions,
                &mut ctx,
                &[Value::int(0)]
            )
            .expect_err("a context that was never started from a command line"),
            nvs_runtime::THROWN
        );
        assert_eq!(
            ctx.take_pending().as_deref(),
            Some(
                "`Core\\Command::completions` has no name to register a script against: this \
                 program was not started from a command line, and a served request is not \
                 something a shell completes"
            ),
            "the sentence says what is missing rather than naming the shell"
        );
    }
}
