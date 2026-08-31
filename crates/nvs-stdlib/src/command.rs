//! `Core\Command` — the compiled command table, as the members that read it.
//!
//! [ADR 0086](../../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 6
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
//! usage: greet <name> [--loud]
//!
//! Greet somebody by name
//!
//! arguments:
//!   <name>
//!
//! options:
//!   --loud  Shout it
//! ```
//!
//! A **positional argument** is `<param>` and an **option** is `[--spelling]`,
//! which is the convention every CLI in the audience's world already reads
//! ([ADR 0080](../../../../docs/adr/0080-the-audience-nvs-is-built-for.md)) —
//! and it is read off the row's own `spellings` rather than off a second field,
//! for the reason `nvs_runtime::commands::CommandArg::spellings` states. An
//! option that declares both a short and a long spelling is *summarized* by its
//! long one in the usage line and lists all of them in the `options:` block: one
//! line that names every spelling is unreadable at four options, and the block
//! below it is where the full answer belongs.
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
//! # Known gaps
//!
//! 1. **§ 6's `completions` is not here.** `completions(Cli\Shell $shell):
//!    string` generates for four shells and reads exactly what `help` reads, so
//!    what it waits on is the `Cli\Shell` enum rather than anything about the
//!    table. `docs/plan/m8.md` owns when.
//! 2. **A page names no types.** The row carries a declared default now
//!    (`nvs_runtime::commands::CommandArg::default`), so `[--retries]` could be
//!    rendered as defaulting to `3`; what it still cannot say is that it is a
//!    `uint`, because § 6's table deliberately does not carry a parameter's
//!    declared type (`nvs_types::commands`'s own gap 1). Saying so needs the
//!    *signature*, which the handler already holds and the row does not, and
//!    inventing a second copy of it in the table is what that gap refuses. The
//!    default alone is not rendered because § 6 asks for neither and a page
//!    naming one of the two reads as though the other were absent from the
//!    declaration.

use nvs_runtime::commands::{ArgConv, Command, CommandArg, CommandTable};
use nvs_runtime::{Fault, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// This class's fully-qualified name, in one place so the registry row and
/// every refusal that names the class cannot drift apart.
pub(crate) const NAME: &str = r"Core\Command";

/// ADR 0086 § 6's generated help, as a registry row. See
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
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Command::help`'s reference card — ADR 0117.
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

/// `Core\Command::run`'s reference card — ADR 0117.
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

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_command_help" => (nvs_core_command_help as *const ()).cast(),
        "nvs_core_command_run" => (nvs_core_command_run as *const ()).cast(),
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
                page_for(row)
            }
        };
        Ok(crate::cli::built(Value::str(NvsStr::new(page.as_bytes()))))
    }
}

/// The exit status a command line nobody can act on answers with — see
/// [`RUN_DOC`]'s `ret`, which is this constant's one home.
const USAGE_STATUS: u64 = 2;

nvs_runtime::nvs_helper! {
    /// `Core\Command::run(): uint` — ADR 0086 § 6's entry point, and its one
    /// deliberate divergence from ADR 0077: this table *dispatches*.
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
        // Asked once, before a word is matched, so that the gap answers as what
        // it is — a program declaring a command this runtime cannot yet call —
        // rather than as a complaint about the argument that reached it.
        if let Some(arg) = row.args.iter().find(|arg| arg.conv == ArgConv::Unconverted) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "`Core\\Command::run` cannot convert an argument into the type `{}`'s `{}` \
                     is declared at yet",
                    row.name, arg.param
                ),
            ));
        }
        let values = match matched(&row, &line[1..]) {
            Ok(values) => values,
            Err(problem) => return usage(ctx, &format!("{problem}\n\n{}", page_for(&row))),
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

/// A usage error: the page on the **diagnostic** channel, and
/// [`USAGE_STATUS`] as the process's status.
///
/// Standard error and not standard output, because a page printed *instead of*
/// what was asked for is not the program's answer — a shell pipeline reading
/// this program's output must not receive a usage page as data. It is the
/// channel `Core\Debug::dump` already writes to (ADR 0092 § 4), which is this
/// tree's only spelling of "not the answer" until ADR 0086 § 3's `write` lands.
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
    match arg.conv {
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
        // Unreachable: the helper refuses a row carrying one before it reads a
        // word, so that the gap answers as a `LogicError` rather than as a
        // usage error about the argument that happened to arrive.
        ArgConv::Unconverted => Err(format!(
            "`{}` has a type no matcher converts yet",
            arg.param
        )),
    }
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

/// One `heading:` and its indented, column-aligned entries, or nothing at all
/// for a heading with no entry under it — an empty section reads as a claim
/// that the command has no arguments, which is what the *absence* says.
fn block(heading: &str, entries: &[(String, Option<&str>)]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let widest = entries
        .iter()
        .map(|(left, _)| left.len())
        .max()
        .unwrap_or(0);
    let mut out = format!("\n{heading}:\n");
    for (left, about) in entries {
        match about {
            Some(about) => out.push_str(&format!("  {left:<widest$}  {about}\n")),
            None => out.push_str(&format!("  {left}\n")),
        }
    }
    out
}

/// One command's page.
fn page_for(row: &Command) -> String {
    let mut page = format!("{}\n", usage_line(row));
    if let Some(about) = &row.about {
        page.push_str(&format!("\n{about}\n"));
    }
    let positionals: Vec<(String, Option<&str>)> = row
        .args
        .iter()
        .filter(|arg| !arg.is_option())
        .map(|arg| (format!("<{}>", arg.param), arg.about.as_deref()))
        .collect();
    let options: Vec<(String, Option<&str>)> = row
        .args
        .iter()
        .filter(|arg| arg.is_option())
        .map(|arg| (arg.spellings.join(", "), arg.about.as_deref()))
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
    let commands: Vec<(String, Option<&str>)> = table
        .rows()
        .iter()
        .map(|row| (row.name.clone(), row.about.as_deref()))
        .collect();
    page.push_str(&block("commands", &commands));
    page
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

    /// ADR 0086 § 6's own example, as a row.
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
    #[test]
    fn a_page_carries_both_blocks_and_every_spelling() {
        assert_eq!(
            page_for(&deploy()),
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
        assert_eq!(page_for(&row), "usage: version\n");
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

    /// ADR 0086 § 6's flag is *given by being written*, so the answer for an
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
            public: true,
            native: false,
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
        ctx.set_runtime_error_class(ErrorClass::new(std::rc::Rc::new(classes), id));
        ctx.set_commands(std::sync::Arc::new(CommandTable::new(vec![deploy()])));
        ctx.set_command_line(line.iter().map(|word| (*word).to_owned()).collect());
        ctx
    }

    /// ADR 0086 § 6's dispatch, end to end: the first word selects a row, the
    /// words past it fill that row's parameters in **declaration** order, and
    /// the handler is reached by the label the compiler wrote into the row —
    /// the whole route `nvs_runtime::call_static` owns, over a class table
    /// built here rather than by a compiler.
    ///
    /// The command line is written `deploy prod -n` and the assertion is that
    /// the option arrived in slot 1: a matcher that handed the handler its
    /// words in the order they were typed would answer the same status.
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
}
