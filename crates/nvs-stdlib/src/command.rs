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
//! 1. **§ 6's `run` and `completions` are not here.** `run(): uint` dispatches
//!    through this same table, and `completions(Cli\Shell $shell): string`
//!    generates for four shells; both read exactly what `help` reads, so what
//!    they wait on is the dispatch itself and the `Cli\Shell` enum, not the
//!    table. `docs/plan/m8.md` owns when.
//! 2. **A page carries no defaults and no types.** § 6's table deliberately does
//!    not carry a parameter's declared type
//!    (`nvs_types::commands`'s own gap 1), so `[--retries]` cannot say that it
//!    is a `uint` defaulting to `3`. Saying so needs the *signature*, which the
//!    handler already holds and the row does not, and inventing a second copy
//!    of it in the table is what that gap refuses.

use nvs_runtime::commands::{Command, CommandTable};
use nvs_runtime::{Fault, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// This class's fully-qualified name, in one place so the registry row and
/// every refusal that names the class cannot drift apart.
pub(crate) const NAME: &str = r"Core\Command";

/// ADR 0086 § 6's generated help, as a registry row. See
/// [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[CoreMethod {
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
    }],
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

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_command_help" => (nvs_core_command_help as *const ()).cast(),
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
                },
                CommandArg {
                    param: "dryRun".to_owned(),
                    spellings: vec!["-n".to_owned(), "--dryRun".to_owned()],
                    about: Some("Print what would happen".to_owned()),
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
}
