//! `nvs code` — questions about one project, answered from the editor's index.
//!
//! `rule:tooling/nvs-code-answers-about-the-project` is the surface. Every
//! answer is read over one [`nvs_lsp::workspace`] through the functions the
//! editor's references, implementation, rename, workspace symbol, call
//! hierarchy and used-by requests call, most of them in [`nvs_lsp::query`], so
//! this command and the editor never disagree. This module decides
//! nothing about the language: it finds the project, turns a [`Site`] into a
//! position a reader can type, escapes what the project wrote, and renders.
//!
//! **Only `rename`, `move`, `delete`, `extract`, `implement`, `inline`,
//! `signature` and `order` write**, through [`planned`] and [`committed`], the one write path: it
//! writes and deletes only inside the project root, and only files whose
//! bytes have not changed since this call first read them. [`write_out`]
//! prints what every write verb but `order` wrote, and [`order()`] prints what
//! it wrote by class.
//!
//! A call loads the editor's index file for the project root from the data
//! folder ([`nvs_lsp::index_file_of`]) and saves it again when it analysed
//! anything, so the editor and every call share one index. The load checks
//! every indexed file and every configuration input by its content hash and
//! analyses, on the workers, each entry whose input changed, so an answer is
//! the one a cold build gives. Without a data folder, or with an index file
//! that does not load, every `.nvs` file under the root is analysed, on the
//! workers too. There is no daemon.
//!
//! **A position** is `path:line:column`, the path relative to the working
//! directory with `/` between its parts, the line and the column counted from
//! 1 and the column in characters. JSON carries the byte `start` and `end`
//! beside it. **Text from the project** — a source line, a doc comment, a
//! signature, a type, a path — has every control character but tab (and the
//! line breaks of a doc comment) written as `\u{..}`, in both renderings,
//! because a file's bytes reach a terminal or an agent's context here.
//!
//! The JSON follows `rule:ide/check-json-is-the-diagnostic-record-as-a-document`
//! as `nvs agent --json` does: `schemaVersion` at the root, every key present,
//! and a failure prints `error` and `nearest` on standard output while the
//! exit status is 1.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::ops::RangeInclusive;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use nvs_diagnostics::{BytePos, SourceFile, SourceMap, canonical_key};
use nvs_hir::members::edit_distance;
use nvs_lsp::extract::Extraction;
use nvs_lsp::order;
use nvs_lsp::query::{self, Outline};
use nvs_lsp::rename::{self, Rewrite};
use nvs_lsp::{DeclKind, Declaration, Site, Unused, Workspace};
use nvs_syntax::ast::{ClassMember, ClassMemberKind, Stmt, StmtKind};
use serde_json::{Map, Value, json};

use crate::agent::{SCHEMA_VERSION, pretty};

/// Print information about the code of the Novis project you are in: where
/// a name is used, what declares it, and which type a variable has.
///
/// The project is the folder of the nearest `nvs.toml` at or above the
/// working directory. If there is none, it is the working directory. Every
/// `.nvs` file in it is read. Only `rename`, `move`, `delete`, `extract`,
/// `implement` and `order` change files.
///
/// The editor and `nvs code` share one index file in the data folder. The
/// index lists every name in the project and every place it is used. A call
/// first compares the content of every file with the index. It analyses
/// only the files that changed, so the result is always current.
///
/// Write a name in full, with its namespace, as `'Shop\Cart::addItem'`.
/// Put it in single quotes, so that the shell keeps the backslashes. A
/// file is a path from the working directory.
// `rule:tooling/nvs-code-answers-about-the-project`.
#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(subcommand)]
    command: Command,
}

/// `nvs code`'s verbs, each a reader of one workspace index
/// (`rule:tooling/nvs-code-answers-about-the-project`).
#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Print where a name is declared, its signature and its doc comment. For
    /// a `Core` name, print the card that `nvs agent show` prints.
    Show {
        /// The name, as `Shop\Cart`, `Shop\Cart::addItem` or
        /// `Shop\Cart::$total`.
        symbol: String,
        /// Print the result as one JSON document. An unknown name prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Print every place a name is written. The declaration comes first. Then
    /// each `use` line and each use follow, in file and line order.
    ///
    /// Each line has the position, the kind (`declared`, `import` or `use`)
    /// and the line of code.
    Refs {
        /// The name, as `Shop\Cart::addItem`. A `Core` name lists the places
        /// where the project uses it.
        symbol: String,
        /// Print the result as one JSON document. An unknown name prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Print the types that extend or implement a type, or the methods that
    /// override a method.
    ///
    /// These lines say `below`. Lines that say `above` show what the type
    /// extends or implements, or which method the method overrides.
    Impls {
        /// The name of a type or a method, as `Shop\Priced`.
        symbol: String,
        /// Print the result as one JSON document. An unknown name prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Print what a file declares, one line each. The constants, properties
    /// and methods of a class are indented under it.
    Outline {
        /// The file, as `src/Cart.nvs`.
        file: String,
        /// Print the result as one JSON document. An unknown file prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Print the type of each variable that a file declares with `var`.
    Types {
        /// The file, as `src/Cart.nvs`. Add `:10` for line 10 only, or
        /// `:10-20` for lines 10 to 20.
        file: String,
        /// Print the result as one JSON document. An unknown file prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Rename a class, a method, a property, a constant, an enum case, a type
    /// alias or a variable everywhere it is used, and save the changed files.
    ///
    /// This is the same rename that the editor does. A method is renamed in
    /// every class that overrides it. A class that is alone in a file with
    /// its name also gets a new file name. A type alias that a class declares
    /// is renamed by its full name, as `Shop\Order::Total`.
    ///
    /// A class can also get a new namespace. Write its full new name, as
    /// `Shop\Billing\Cart`. The command changes every `use` line and moves the
    /// file to the folder that `autoload` reads for the new name.
    ///
    /// With `--from`, the command reads a list of renames from a file and
    /// does all of them in one change.
    ///
    /// The command prints one line for each changed file. If the rename is
    /// not possible, it prints why, changes nothing and fails.
    Rename {
        /// What to rename. Write a name, as `Shop\Cart::addItem`, or a
        /// position, as `src/Cart.nvs:12:9`. A local variable has no name
        /// outside its function, so use its position.
        #[arg(required_unless_present = "from")]
        target: Option<String>,
        /// The new name, as `addProduct`. For a variable or a property, the
        /// `$` is optional. For a class, a full name, as `Shop\Billing\Cart`,
        /// moves it to that namespace.
        #[arg(required_unless_present = "from")]
        new_name: Option<String>,
        /// Read the renames from this file. Each line has what to rename and
        /// the new name, separated by a space. Empty lines and lines that
        /// start with `#` are skipped. Each position is read in the files
        /// before any change. Two names can swap. If one rename is not
        /// possible, nothing is changed.
        #[arg(long, value_name = "FILE", conflicts_with_all = ["target", "new_name"])]
        from: Option<String>,
        /// Print each change, and save nothing. Each line has the position,
        /// the old text and the new text.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `written` field is
        /// `true` when the files were saved.
        #[arg(long)]
        json: bool,
    },
    /// Move a file or a directory, or move a member of a class to another
    /// class.
    ///
    /// `nvs code move src/Cart.nvs src/Basket.nvs` moves a file. Every
    /// `require` that names it changes too. This is the same file move that
    /// the editor does. A directory moves with everything in it. The command
    /// does not create a directory.
    ///
    /// `nvs code move Shop\Cart::total Shop\Order` moves a member. A static
    /// method, a static property or a constant can move to any class. Every
    /// `Cart::total` becomes `Order::total`. A method or a property that is
    /// not static can only move to a parent class or a child class. Its calls
    /// do not change. If the new code would not compile, nothing is changed.
    ///
    /// The `autoload` setting finds a class by its path. So when a moved file
    /// is found that way, its class gets the name of the new path. A new
    /// folder changes the namespace, and a new file name changes the class
    /// name. If no `autoload` folder contains the new path, nothing is moved.
    ///
    /// The command prints one line for each changed file and one for the
    /// move. If the move is not possible, it prints why, changes nothing and
    /// fails.
    Move {
        /// The file or directory to move, as `src/Cart.nvs` or `src/Shop`.
        /// Or the member to move, as `Shop\Cart::total`.
        from: String,
        /// The new path, as `src/Basket.nvs`. Nothing may exist there yet,
        /// and its directory must exist. For a member, the class it moves
        /// to, as `Shop\Order`.
        to: String,
        /// Print each change and the move, and save nothing. Each line has
        /// the position, the old text and the new text.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `written` field is
        /// `true` when the files were saved and moved.
        #[arg(long)]
        json: bool,
    },
    /// Put the constants, properties and methods of each class and interface
    /// in the standard order, and save the changed files.
    ///
    /// This is the same change as the editor's "Order class members" action.
    /// Types come first, then constants, then properties, then methods. In
    /// each group, everything static comes first. Then `public` comes
    /// before `protected`, and `protected` before `private`. The constructor
    /// is the first method that is not static. Two that are equal in this
    /// order keep the order you wrote. Each changed file is
    /// also formatted as `nvs fmt` formats it.
    ///
    /// The command prints one line for each changed file. A class with
    /// `#[Core\Json\Derive]` gets a note when its JSON key order changes. A
    /// file with a syntax error is not changed, and a warning names it.
    Order {
        /// The files or directories to order, as `src/Cart.nvs` or `src`.
        /// With no path, every `.nvs` file in the project is ordered.
        paths: Vec<String>,
        /// Save nothing. Print each class that is not in order, and fail if
        /// there is one.
        #[arg(long, conflicts_with = "dry_run")]
        check: bool,
        /// Print each class that would change, and the notes. Save nothing.
        #[arg(long)]
        dry_run: bool,
        /// Print the files, the classes and the notes as one JSON document.
        /// The `written` field is `true` when the files were saved.
        #[arg(long)]
        json: bool,
    },
    /// Print every class, method, property, constant and enum case whose
    /// name contains a text. Upper and lower case count as the same.
    ///
    /// Each line has the position, the kind and the full name. The lines are
    /// sorted by name. `Core` names are not searched. Use `nvs agent find`
    /// for them.
    Find {
        /// The text to look for, as `item`. It finds `Shop\Cart::addItem`.
        query: String,
        /// Print the result as one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Print every class, method, property, constant and enum case that
    /// nothing in the project uses.
    ///
    /// Each line has the position, the kind and the full name. Some code is
    /// used without being named in the source, and it is not printed: a
    /// route, a test, a constructor, a declaration with an attribute, a
    /// method that implements an interface of `Core`, and a public property.
    /// A name that is written in a string or in an `nvs.toml` file also
    /// counts as used. In a package, every `public` and `protected`
    /// declaration is used by the package's users, so only `private` ones
    /// are printed. The command does not fail when it prints something.
    Unused {
        /// The files or directories to read, as `src/Cart.nvs` or `src`.
        /// With no path, every `.nvs` file in the project is read.
        paths: Vec<String>,
        /// Print the result as one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Delete a class, a method, a property, a constant or an enum case that
    /// nothing in the project uses, and save the changed file.
    ///
    /// The command deletes the declaration with its `///` comment, its
    /// attributes and one blank line. A class that is alone in its file
    /// deletes the file. The command deletes only what `nvs code unused`
    /// prints. A method that overrides another method, or that another
    /// method overrides, is not deleted.
    ///
    /// The command prints the changed or deleted file. If the delete is not
    /// possible, it prints why, changes nothing and fails.
    Delete {
        /// What to delete, as `Shop\Cart::oldTotal` or `Shop\OldCart`.
        name: String,
        /// Print each change, and save nothing.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `written` field is
        /// `true` when the files were saved.
        #[arg(long)]
        json: bool,
    },
    /// Move a part of the code into a new variable or a new method, or the
    /// methods of a class into a new interface, and save the changed files.
    ///
    /// These are the same changes as the editor's "Extract to variable",
    /// "Extract to method" and "Extract interface" actions. The command prints
    /// each changed file. If the change is not possible, it prints why,
    /// changes nothing and fails.
    Extract {
        #[command(subcommand)]
        what: Extracted,
    },
    /// Write every method that a class must have and does not have yet, and
    /// save the changed file.
    ///
    /// The methods come from the interfaces of the class and from its
    /// `abstract` parent classes. Each new method has the signature of the
    /// method it implements. Its body throws a `LogicError`, so you can find
    /// it and write the real code. This is the same change as the editor's
    /// "Implement the missing methods" action.
    ///
    /// The command prints the changed file. If the change is not possible,
    /// it prints why, changes nothing and fails.
    Implement {
        /// The class, as `Shop\Cart`.
        class: String,
        /// Print each change, and save nothing.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `written` field is
        /// `true` when the files were saved.
        #[arg(long)]
        json: bool,
    },
    /// Replace a variable or a method call with its value, and save the
    /// changed files.
    ///
    /// A position on a local variable replaces each read of the variable
    /// with its value, and deletes the variable. A method name, or a
    /// position on the method's name, replaces every call with the body of
    /// the method, and deletes the method. A position on one call replaces
    /// only that call, and keeps the method. This is the same change as the
    /// editor's "Inline" actions.
    ///
    /// The command does not inline when the code would run in another
    /// order, or run a different number of times. For example, each
    /// argument of an inlined call must be a variable or a literal. The
    /// command prints the changed files. If the change is not possible, it
    /// prints why, changes nothing and fails.
    Inline {
        /// What to inline, as `Shop\Cart::total` or `src/Cart.nvs:12:9`.
        target: String,
        /// Print each change, and save nothing.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `written` field is
        /// `true` when the files were saved.
        #[arg(long)]
        json: bool,
    },
    /// Change the parameters of a method, and save the changed files.
    ///
    /// The list is the whole new parameter list, as you write it in the
    /// method. Parameters keep their name, so a parameter with an old name
    /// is the old parameter. It may get another type, default or place. A
    /// new parameter needs a default or a `--value`. The value is written
    /// at every call. A removed parameter's argument is deleted at every
    /// call. Every method that overrides the method, or that it overrides,
    /// gets the same list.
    ///
    /// The command does not change a call when its code would run in
    /// another order, or not at all. For example, a removed argument may
    /// not call a function. The command prints the changed files. If the
    /// change is not possible, it prints why, changes nothing and fails.
    Signature {
        /// The method, as `Shop\Cart::addItem`.
        method: String,
        /// The new parameter list, as `int $count, string $note = ''`.
        parameters: String,
        /// The value of a new parameter at every call, as `$note='gift'`.
        /// You can give this more than once.
        #[arg(long = "value", value_name = "$NAME=VALUE")]
        values: Vec<String>,
        /// Print each change, and save nothing.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `written` field is
        /// `true` when the files were saved.
        #[arg(long)]
        json: bool,
    },
    /// Print the methods that call a method, and the methods it calls.
    ///
    /// The callers come first. Each method has its own line, with `caller`
    /// or `callee`. Under it, each call has an indented line with the
    /// position and the line of code. A test that sends a request to a
    /// route calls the method that handles the route.
    ///
    /// With several methods, each method gets a line with its name. Its
    /// own lines follow, indented by two spaces.
    Calls {
        /// The methods, as `Shop\Cart::addItem`. If one name is not a method,
        /// the command prints nothing else and fails.
        #[arg(value_name = "METHOD", required = true)]
        symbols: Vec<String>,
        /// Print the result as one JSON document. An unknown method prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
    /// Print the routes and the tests that reach a method through a chain of
    /// calls.
    ///
    /// Each route has a line that starts with `route`, and each test has a
    /// line that starts with `test`. A last line starts with `at least` when
    /// a call that is only known when the program runs may also reach the
    /// method.
    ///
    /// With several methods, each method gets a line with its name. Its
    /// own lines follow, indented by two spaces.
    Reach {
        /// The methods, as `Shop\Cart::addItem`. If one name is not a method,
        /// the command prints nothing else and fails.
        #[arg(value_name = "METHOD", required = true)]
        symbols: Vec<String>,
        /// Print the result as one JSON document. An unknown method prints an
        /// `error` and a `nearest` list, and the command fails.
        #[arg(long)]
        json: bool,
    },
}

/// What `nvs code extract` writes.
#[derive(clap::Subcommand, Debug)]
enum Extracted {
    /// Move an expression into a new local variable above its statement.
    ///
    /// The variable gets the type of the expression. The expression is
    /// replaced by the variable.
    Variable {
        /// The expression, as `src/Cart.nvs:12:9-12:30`. This is the file,
        /// the line and column of the first character, and the line and
        /// column of the last character. Both columns are part of the
        /// expression.
        range: String,
        /// The name of the variable, as `total`. Without it, the command
        /// takes a name from the expression.
        #[arg(long)]
        name: Option<String>,
        /// Print each change, and save nothing.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `written` field is
        /// `true` when the files were saved.
        #[arg(long)]
        json: bool,
    },
    /// Move an expression or some whole statements into a new `private`
    /// method of the same class.
    ///
    /// The new method comes after the method the code is in. Each variable
    /// that the code reads becomes a parameter. The code is replaced by a
    /// call to the new method.
    Method {
        /// The code, as `src/Cart.nvs:12:9-14:10`. This is the file, the
        /// line and column of the first character, and the line and column of
        /// the last character. Both columns are part of the code.
        range: String,
        /// The name of the method, as `computeTotal`. Without it, the
        /// method is named `extracted`.
        #[arg(long)]
        name: Option<String>,
        /// Print each change, and save nothing.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `written` field is
        /// `true` when the files were saved.
        #[arg(long)]
        json: bool,
    },
    /// Write a new interface with the public methods of a class. The class
    /// then implements the interface.
    ///
    /// The interface gets its own file. The `autoload` setting gives its
    /// path. Each method keeps its parameters, its return type and its `///`
    /// comment. No other code changes to use the interface.
    Interface {
        /// The class, as `Shop\Cart`.
        class: String,
        /// The name of the interface, as `Priced`. A name without a
        /// namespace is in the namespace of the class. No type may have
        /// this name yet.
        name: String,
        /// The methods to take, as `total,addItem`. Without it, the
        /// interface gets every public method that is not static, except
        /// the constructor.
        #[arg(long, value_delimiter = ',')]
        methods: Option<Vec<String>>,
        /// Print each change and the path of the new file, and save nothing.
        #[arg(long)]
        dry_run: bool,
        /// Print the changes as one JSON document. The `created` field lists
        /// the new file. The `written` field is `true` when the files were
        /// saved.
        #[arg(long)]
        json: bool,
    },
}

impl Args {
    /// Runs the `nvs code` verb clap parsed.
    pub(crate) fn run(self) -> ExitCode {
        match self.command {
            Command::Show { symbol, json } => show(&symbol, json),
            Command::Refs { symbol, json } => refs(&symbol, json),
            Command::Impls { symbol, json } => impls(&symbol, json),
            Command::Outline { file, json } => outline(&file, json),
            Command::Types { file, json } => types(&file, json),
            Command::Rename {
                target,
                new_name,
                from,
                dry_run,
                json,
            } => match (from, target, new_name) {
                (Some(from), _, _) => rename_from(&from, dry_run, json),
                (None, Some(target), Some(new_name)) => rename(&target, &new_name, dry_run, json),
                // clap requires both positional arguments without `--from`.
                (None, _, _) => ExitCode::FAILURE,
            },
            Command::Move {
                from,
                to,
                dry_run,
                json,
            } if from.contains("::") => move_member(&from, &to, dry_run, json),
            Command::Move {
                from,
                to,
                dry_run,
                json,
            } => move_path(&from, &to, dry_run, json),
            Command::Order {
                paths,
                check,
                dry_run,
                json,
            } => order(&paths, check, dry_run, json),
            Command::Find { query, json } => find(&query, json),
            Command::Unused { paths, json } => unused(&paths, json),
            Command::Delete {
                name,
                dry_run,
                json,
            } => delete(&name, dry_run, json),
            Command::Extract { what } => match what {
                Extracted::Variable {
                    range,
                    name,
                    dry_run,
                    json,
                } => extract(&range, Extraction::Variable, name.as_deref(), dry_run, json),
                Extracted::Method {
                    range,
                    name,
                    dry_run,
                    json,
                } => extract(&range, Extraction::Method, name.as_deref(), dry_run, json),
                Extracted::Interface {
                    class,
                    name,
                    methods,
                    dry_run,
                    json,
                } => extract_interface(&class, &name, methods.as_deref(), dry_run, json),
            },
            Command::Implement {
                class,
                dry_run,
                json,
            } => implement(&class, dry_run, json),
            Command::Inline {
                target,
                dry_run,
                json,
            } => inline(&target, dry_run, json),
            Command::Signature {
                method,
                parameters,
                values,
                dry_run,
                json,
            } => change_signature(&method, &parameters, &values, dry_run, json),
            Command::Calls { symbols, json } => calls(&symbols, json),
            Command::Reach { symbols, json } => reach(&symbols, json),
        }
    }
}

/// How many names or files an unknown argument lists as nearest.
const NEAREST: usize = 5;

/// The working directory, the project around it and the workspace built over
/// that project, with each file's text read once.
struct Project {
    cwd: PathBuf,
    workspace: Workspace,
    texts: HashMap<PathBuf, String>,
}

impl Project {
    /// Finds the project and indexes it from the data folder's index file,
    /// saving that file again, and reports on standard error how many of its
    /// files have errors.
    fn open() -> Result<Self, ExitCode> {
        let cwd = std::env::current_dir().map_err(|err| {
            eprintln!("error: the working directory cannot be read: {err}");
            ExitCode::FAILURE
        })?;
        let cwd = canonical_key(&cwd);
        let root = project_root(&cwd);
        let file = nvs_lsp::index_file_of(&root);
        let workspace = nvs_lsp::workspace_from(&root, file.as_deref());
        match workspace.index().files_with_errors() {
            0 => {}
            1 => eprintln!(
                "warning: 1 file in this project has errors. A use inside it may be missing."
            ),
            n => eprintln!(
                "warning: {n} files in this project have errors. A use inside them may be missing."
            ),
        }
        Ok(Self {
            cwd,
            workspace,
            texts: HashMap::new(),
        })
    }

    /// The text of `path`, read once, and empty for a file that cannot be read.
    fn text(&mut self, path: &Path) -> &str {
        self.texts
            .entry(path.to_owned())
            .or_insert_with(|| std::fs::read_to_string(path).unwrap_or_default())
    }

    /// `site` as a reader types it, and the source line it is on.
    fn place(&mut self, site: &Site) -> Place {
        let path = escaped(&relative(&site.path, &self.cwd));
        let text = self.text(&site.path);
        let before = text.get(..site.start as usize).unwrap_or_default();
        let line_start = before.rfind('\n').map_or(0, |at| at + 1);
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |at| line_start + at);
        Place {
            path,
            line: before.matches('\n').count() + 1,
            column: before[line_start..].chars().count() + 1,
            start: site.start,
            end: site.end,
            source: escaped(
                text[line_start..line_end]
                    .trim_start()
                    .trim_end_matches('\r'),
            ),
        }
    }

    /// A file argument, resolved against the working directory, when the
    /// project indexed it.
    fn file(&self, written: &str) -> Option<PathBuf> {
        let path = canonical_key(&self.cwd.join(written));
        self.workspace.index().holds(&path).then_some(path)
    }

    /// The bytes of `path`, read once, which every later position in it is
    /// counted in. A file that cannot be read is an error.
    fn snapshot(&mut self, path: &Path) -> Result<&str, String> {
        if !self.texts.contains_key(path) {
            let text = std::fs::read_to_string(path).map_err(|err| {
                format!(
                    "`{}` cannot be read: {err}",
                    escaped(&relative(path, &self.cwd))
                )
            })?;
            self.texts.insert(path.to_owned(), text);
        }
        Ok(self.text(path))
    }

    /// `path` as a position prints it.
    fn shown(&self, path: &Path) -> String {
        escaped(&relative(path, &self.cwd))
    }

    /// The project's files nearest to `written`, as positions print them.
    fn nearest_files(&self, written: &str) -> Vec<String> {
        let wanted = written.replace('\\', "/");
        let mut ranked: Vec<(usize, String)> = self
            .workspace
            .index()
            .files()
            .map(|path| {
                let shown = relative(path, &self.cwd);
                (edit_distance(&wanted, &shown), escaped(&shown))
            })
            .collect();
        ranked.sort();
        ranked
            .into_iter()
            .take(NEAREST)
            .map(|(_, shown)| shown)
            .collect()
    }

    /// The project's names nearest to `symbol`.
    fn nearest_names(&self, symbol: &str) -> Vec<String> {
        query::nearest(self.workspace.index(), symbol, NEAREST)
            .into_iter()
            .map(escaped)
            .collect()
    }
}

/// Where something is written, as a reader types it.
struct Place {
    path: String,
    line: usize,
    column: usize,
    start: u32,
    end: u32,
    /// The whole source line, its indentation removed.
    source: String,
}

impl Place {
    fn position(&self) -> String {
        format!("{}:{}:{}", self.path, self.line, self.column)
    }

    /// The position's keys of a JSON record.
    fn json(&self) -> Map<String, Value> {
        let mut out = Map::new();
        out.insert("path".into(), Value::from(self.path.clone()));
        out.insert("line".into(), Value::from(self.line));
        out.insert("column".into(), Value::from(self.column));
        out.insert("start".into(), Value::from(self.start));
        out.insert("end".into(), Value::from(self.end));
        out
    }
}

/// `nvs code show`: where a name is declared, its signature and its doc
/// comment, or a `Core` name's card as `nvs agent show` prints it.
pub(crate) fn show(symbol: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let card = query::declaration(&project.workspace, symbol)
        .map(|card| (card.declaration.clone(), card.doc));
    let Some((declared, doc)) = card else {
        if crate::agent::has_card(symbol) {
            return crate::agent::show(symbol, json);
        }
        let nearest = project.nearest_names(symbol);
        return unknown(
            &format!("no name `{}` in this project", escaped(symbol)),
            &nearest,
            json,
        );
    };
    let place = project.place(&declared.site);
    let signature = signature(&project.workspace, &declared).map(|text| escaped(&text));
    let doc = doc.as_deref().map(escaped_lines);
    if json {
        let mut out = Map::new();
        out.insert("schemaVersion".into(), Value::from(SCHEMA_VERSION));
        out.insert("symbol".into(), Value::from(escaped(&declared.symbol)));
        out.insert("kind".into(), Value::from(kind(declared.kind)));
        out.extend(place.json());
        out.insert(
            "signature".into(),
            signature.map_or(Value::Null, Value::from),
        );
        out.insert("doc".into(), doc.map_or(Value::Null, Value::from));
        print!("{}", pretty(Value::Object(out)));
        return ExitCode::SUCCESS;
    }
    println!(
        "{}  {}, declared at {}",
        escaped(&declared.symbol),
        kind(declared.kind),
        place.position()
    );
    if let Some(signature) = signature {
        println!("{signature}");
    }
    if let Some(doc) = doc {
        println!("{doc}");
    }
    ExitCode::SUCCESS
}

/// `nvs code refs`: the declaration, then every import and use, in path and
/// then line order.
pub(crate) fn refs(symbol: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let found: Vec<(&'static str, Site)> =
        query::references(project.workspace.index(), symbol, true)
            .iter()
            .map(|found| (found.kind.label(), found.site.clone()))
            .collect();
    if found.is_empty() && !known(&project, symbol) {
        let nearest = project.nearest_names(symbol);
        return unknown(
            &format!("no name `{}` in this project", escaped(symbol)),
            &nearest,
            json,
        );
    }
    let rows: Vec<(&str, Place)> = found
        .iter()
        .map(|(kind, site)| (*kind, project.place(site)))
        .collect();
    if json {
        let references: Vec<Value> = rows
            .iter()
            .map(|(kind, place)| {
                let mut record = place.json();
                record.insert("kind".into(), Value::from(*kind));
                record.insert("text".into(), Value::from(place.source.clone()));
                Value::Object(record)
            })
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "symbol": escaped(symbol),
                "references": references,
            }))
        );
        return ExitCode::SUCCESS;
    }
    let width = rows
        .iter()
        .map(|(_, place)| place.position().len())
        .max()
        .unwrap_or(0);
    for (kind, place) in &rows {
        println!("{:<width$}  {kind:<8}  {}", place.position(), place.source);
    }
    ExitCode::SUCCESS
}

/// `nvs code impls`: what sits below and above a type or a method.
pub(crate) fn impls(symbol: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let found = query::implementations(project.workspace.index(), symbol);
    let rows: Vec<(&str, Declaration)> = found
        .below
        .iter()
        .map(|declared| ("below", (*declared).clone()))
        .chain(
            found
                .above
                .iter()
                .map(|declared| ("above", (*declared).clone())),
        )
        .collect();
    if rows.is_empty() && !known(&project, symbol) {
        let nearest = project.nearest_names(symbol);
        return unknown(
            &format!("no name `{}` in this project", escaped(symbol)),
            &nearest,
            json,
        );
    }
    let placed: Vec<(&str, &Declaration, Place)> = rows
        .iter()
        .map(|(relation, declared)| (*relation, declared, project.place(&declared.site)))
        .collect();
    if json {
        let record = |declared: &Declaration, place: &Place| {
            let mut record = place.json();
            record.insert("symbol".into(), Value::from(escaped(&declared.symbol)));
            record.insert("kind".into(), Value::from(kind(declared.kind)));
            Value::Object(record)
        };
        let side = |wanted: &str| -> Vec<Value> {
            placed
                .iter()
                .filter(|(relation, _, _)| *relation == wanted)
                .map(|(_, declared, place)| record(declared, place))
                .collect()
        };
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "symbol": escaped(symbol),
                "below": side("below"),
                "above": side("above"),
            }))
        );
        return ExitCode::SUCCESS;
    }
    let width = placed
        .iter()
        .map(|(_, _, place)| place.position().len())
        .max()
        .unwrap_or(0);
    for (relation, declared, place) in &placed {
        println!(
            "{:<width$}  {relation}  {}",
            place.position(),
            escaped(&declared.symbol)
        );
    }
    ExitCode::SUCCESS
}

/// `nvs code outline`: what a file declares, members under their owner.
pub(crate) fn outline(file: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let Some(path) = project.file(file) else {
        let nearest = project.nearest_files(file);
        return unknown(
            &format!("no file `{}` in this project", escaped(file)),
            &nearest,
            json,
        );
    };
    let index = project.workspace.index();
    let entries: Vec<(usize, Declaration)> = flatten(&query::outline(index, &path), 0);
    let rows: Vec<(usize, &Declaration, Place)> = entries
        .iter()
        .map(|(depth, declared)| (*depth, declared, project.place(&declared.site)))
        .collect();
    if json {
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "path": escaped(&relative(&path, &project.cwd)),
                "declarations": nested_json(&rows, 0, &mut 0),
            }))
        );
        return ExitCode::SUCCESS;
    }
    let width = rows
        .iter()
        .map(|(_, _, place)| place.position().len())
        .max()
        .unwrap_or(0);
    for (depth, declared, place) in &rows {
        println!(
            "{:<width$}  {}{}  {}",
            place.position(),
            "  ".repeat(*depth),
            kind(declared.kind),
            escaped(&declared.symbol)
        );
    }
    ExitCode::SUCCESS
}

/// An outline in source order, each declaration with how deep it sits.
fn flatten(outline: &[Outline<'_>], depth: usize) -> Vec<(usize, Declaration)> {
    outline
        .iter()
        .flat_map(|entry| {
            std::iter::once((depth, entry.declaration.clone()))
                .chain(flatten(&entry.members, depth + 1))
        })
        .collect()
}

/// The flattened rows from `*next` on that sit at `depth`, each with the rows
/// below it under `members`.
fn nested_json(
    rows: &[(usize, &Declaration, Place)],
    depth: usize,
    next: &mut usize,
) -> Vec<Value> {
    let mut out = Vec::new();
    while let Some((at, declared, place)) = rows.get(*next) {
        if *at != depth {
            break;
        }
        *next += 1;
        let mut record = place.json();
        record.insert("symbol".into(), Value::from(escaped(&declared.symbol)));
        record.insert("kind".into(), Value::from(kind(declared.kind)));
        record.insert(
            "members".into(),
            Value::Array(nested_json(rows, depth + 1, next)),
        );
        out.push(Value::Object(record));
    }
    out
}

/// `nvs code types`: each variable declared with `var` in a file or a range
/// of its lines, with the type the checker inferred.
pub(crate) fn types(spec: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let (file, lines) = split_lines(spec);
    let typed = project
        .file(file)
        .and_then(|path| query::types(&project.workspace, &path, lines).map(|typed| (path, typed)));
    let Some((path, typed)) = typed else {
        let nearest = project.nearest_files(file);
        return unknown(
            &format!("no file `{}` in this project", escaped(file)),
            &nearest,
            json,
        );
    };
    let rows: Vec<(String, String, Place)> = typed
        .iter()
        .map(|binding| {
            (
                escaped(&binding.name),
                escaped(&binding.ty),
                project.place(&binding.site),
            )
        })
        .collect();
    if json {
        let records: Vec<Value> = rows
            .iter()
            .map(|(name, ty, place)| {
                let mut record = place.json();
                record.insert("name".into(), Value::from(name.clone()));
                record.insert("type".into(), Value::from(ty.clone()));
                Value::Object(record)
            })
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "path": escaped(&relative(&path, &project.cwd)),
                "types": records,
            }))
        );
        return ExitCode::SUCCESS;
    }
    let width = rows
        .iter()
        .map(|(_, _, place)| place.position().len())
        .max()
        .unwrap_or(0);
    for (name, ty, place) in &rows {
        println!("{:<width$}  {name}: {ty}", place.position());
    }
    ExitCode::SUCCESS
}

/// `nvs code find`: every declaration whose own name contains `text`, as the
/// editor's workspace symbol search matches it.
pub(crate) fn find(text: &str, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let found: Vec<Declaration> = project
        .workspace
        .index()
        .matching(text)
        .into_iter()
        .cloned()
        .collect();
    let rows: Vec<(&Declaration, Place)> = found
        .iter()
        .map(|declared| (declared, project.place(&declared.site)))
        .collect();
    if json {
        let records: Vec<Value> = rows
            .iter()
            .map(|(declared, place)| {
                let mut record = place.json();
                record.insert("symbol".into(), Value::from(escaped(&declared.symbol)));
                record.insert("kind".into(), Value::from(kind(declared.kind)));
                Value::Object(record)
            })
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "query": escaped(text),
                "declarations": records,
            }))
        );
        return ExitCode::SUCCESS;
    }
    let width = rows
        .iter()
        .map(|(_, place)| place.position().len())
        .max()
        .unwrap_or(0);
    for (declared, place) in &rows {
        println!(
            "{:<width$}  {}  {}",
            place.position(),
            kind(declared.kind),
            escaped(&declared.symbol)
        );
    }
    ExitCode::SUCCESS
}

/// `nvs code unused`: every declaration in `paths` that nothing uses, as
/// [`nvs_lsp::SymbolIndex::unused`] decides it, in path and source order.
pub(crate) fn unused(paths: &[String], json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let files = match ordered_files(&project, paths) {
        Ok(files) => files,
        Err((message, nearest)) => return unknown(&message, &nearest, json),
    };
    let index = project.workspace.index();
    let scope = Unused::of_project(project.workspace.root(), index);
    let found: Vec<Declaration> = files
        .iter()
        .flat_map(|file| index.unused(file, &scope))
        .cloned()
        .collect();
    let rows: Vec<(&Declaration, Place)> = found
        .iter()
        .map(|declared| (declared, project.place(&declared.site)))
        .collect();
    if json {
        let records: Vec<Value> = rows
            .iter()
            .map(|(declared, place)| {
                let mut record = place.json();
                record.insert("symbol".into(), Value::from(escaped(&declared.symbol)));
                record.insert("kind".into(), Value::from(kind(declared.kind)));
                Value::Object(record)
            })
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "declarations": records,
            }))
        );
        return ExitCode::SUCCESS;
    }
    let width = rows
        .iter()
        .map(|(_, place)| place.position().len())
        .max()
        .unwrap_or(0);
    for (declared, place) in &rows {
        println!(
            "{:<width$}  {}  {}",
            place.position(),
            kind(declared.kind),
            escaped(&declared.symbol)
        );
    }
    ExitCode::SUCCESS
}

/// `nvs code delete`: the editor's deletion of a declaration nothing uses
/// ([`nvs_lsp::delete::delete`]), written to disk through [`write_out`], or
/// printed with `--dry-run`.
pub(crate) fn delete(name: &str, dry_run: bool, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    if project.workspace.index().declaration(name).is_none() {
        let nearest = project.nearest_names(name);
        return unknown(
            &format!("no name `{}` in this project", escaped(name)),
            &nearest,
            json,
        );
    }
    let scope = Unused::of_project(project.workspace.root(), project.workspace.index());
    let deleted = nvs_lsp::delete::delete(
        project.workspace.documents(),
        project.workspace.index(),
        name,
        &scope,
    );
    match deleted {
        Ok(deleted) => {
            let files: Vec<PathBuf> = deleted.file.into_iter().collect();
            write_out(
                &mut project,
                &deleted.rewrites,
                &[],
                &Whole {
                    deleted: &files,
                    created: &[],
                },
                None,
                dry_run,
                json,
            )
        }
        Err(message) => unknown(&escaped(&message), &[], json),
    }
}

/// `nvs code extract variable` and `nvs code extract method`: the editor's
/// extraction over a range ([`query::extract`]), written to disk through
/// [`write_out`], or printed with `--dry-run`. A selection that cannot be
/// extracted fails with the reason.
pub(crate) fn extract(
    range: &str,
    which: Extraction,
    name: Option<&str>,
    dry_run: bool,
    json: bool,
) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let Some((file, from, (line, column))) = split_range(range) else {
        let message = format!(
            "`{}` is not a range. Write it as `src/Cart.nvs:12:9-12:30`.",
            escaped(range)
        );
        return unknown(&message, &[], json);
    };
    let Some(path) = project.file(file) else {
        let nearest = project.nearest_files(file);
        return unknown(
            &format!("no file `{}` in this project", escaped(file)),
            &nearest,
            json,
        );
    };
    // The last column is part of the range, and the engine's end is the
    // character after it.
    let to = (line, column.saturating_add(1));
    match query::extract(&project.workspace, &path, from, to, which, name) {
        Ok(rewrites) => write_out(
            &mut project,
            &rewrites,
            &[],
            &Whole::NONE,
            None,
            dry_run,
            json,
        ),
        Err(message) => unknown(&escaped(&message), &[], json),
    }
}

/// `nvs code extract interface`: a new interface with the methods of a class
/// ([`query::extract_interface`]), its file created and the class's file
/// changed through [`write_out`], or printed with `--dry-run`.
pub(crate) fn extract_interface(
    class: &str,
    name: &str,
    methods: Option<&[String]>,
    dry_run: bool,
    json: bool,
) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    if project.workspace.index().declaration(class).is_none() {
        let nearest = project.nearest_names(class);
        return unknown(
            &format!("no name `{}` in this project", escaped(class)),
            &nearest,
            json,
        );
    }
    match query::extract_interface(&project.workspace, class, name, methods) {
        Ok(extracted) => {
            let created = [(extracted.path, extracted.text)];
            write_out(
                &mut project,
                &extracted.rewrites,
                &[],
                &Whole {
                    deleted: &[],
                    created: &created,
                },
                None,
                dry_run,
                json,
            )
        }
        Err(message) => unknown(&escaped(&message), &[], json),
    }
}

/// `nvs code implement`: the editor's *Implement the missing methods* for a
/// class by name ([`query::implement`]), written to disk through
/// [`write_out`], or printed with `--dry-run`.
pub(crate) fn implement(class: &str, dry_run: bool, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    if project.workspace.index().declaration(class).is_none() {
        let nearest = project.nearest_names(class);
        return unknown(
            &format!("no name `{}` in this project", escaped(class)),
            &nearest,
            json,
        );
    }
    match query::implement(&project.workspace, class) {
        Ok(rewrites) => write_out(
            &mut project,
            &rewrites,
            &[],
            &Whole::NONE,
            None,
            dry_run,
            json,
        ),
        Err(message) => unknown(&escaped(&message), &[], json),
    }
}

/// `nvs code inline`: the editor's inline of a local, a call or a method
/// ([`query::inline`]), written to disk through [`write_out`], or printed with
/// `--dry-run`. A target that cannot be inlined fails with the reason.
pub(crate) fn inline(target: &str, dry_run: bool, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let inlined = if let Some((file, line, column)) = split_position(target) {
        let Some(path) = project.file(file) else {
            let nearest = project.nearest_files(file);
            return unknown(
                &format!("no file `{}` in this project", escaped(file)),
                &nearest,
                json,
            );
        };
        query::inline(&project.workspace, &query::Inlined::At(&path, line, column))
    } else {
        if project.workspace.index().declaration(target).is_none() {
            let nearest = project.nearest_names(target);
            return unknown(
                &format!("no name `{}` in this project", escaped(target)),
                &nearest,
                json,
            );
        }
        query::inline(&project.workspace, &query::Inlined::Method(target))
    };
    match inlined {
        Ok(rewrites) => write_out(
            &mut project,
            &rewrites,
            &[],
            &Whole::NONE,
            None,
            dry_run,
            json,
        ),
        Err(message) => unknown(&escaped(&message), &[], json),
    }
}

/// `nvs code signature`: a method's new parameter list in its whole override
/// family and at every call ([`query::signature`]), written to disk through
/// [`write_out`], or printed with `--dry-run`. Each `--value` is
/// `$name=<expression>`.
pub(crate) fn change_signature(
    symbol: &str,
    parameters: &str,
    values: &[String],
    dry_run: bool,
    json: bool,
) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let mut pairs = Vec::new();
    for value in values {
        match value.split_once('=') {
            Some((name, expr)) if name.trim().starts_with('$') && !expr.trim().is_empty() => {
                pairs.push((name.trim().to_owned(), expr.trim().to_owned()));
            }
            _ => {
                return unknown(
                    &format!(
                        "`--value {}` is not written as `$name=value`",
                        escaped(value)
                    ),
                    &[],
                    json,
                );
            }
        }
    }
    if !method(&project, symbol) {
        return unknown_method(&project, symbol, json);
    }
    match query::signature(&project.workspace, symbol, parameters, &pairs) {
        Ok(rewrites) => write_out(
            &mut project,
            &rewrites,
            &[],
            &Whole::NONE,
            None,
            dry_run,
            json,
        ),
        Err(message) => unknown(&escaped(&message), &[], json),
    }
}

/// `nvs code move <Class::member> <Class>`: a member moved into another class
/// ([`query::move_member`]), written to disk through [`write_out`], or printed
/// with `--dry-run`.
pub(crate) fn move_member(from: &str, to: &str, dry_run: bool, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let index = project.workspace.index();
    let property = from
        .rsplit_once("::")
        .map(|(class, name)| format!("{class}::${name}"));
    if index.declaration(from).is_none()
        && property.is_none_or(|property| index.declaration(&property).is_none())
    {
        let nearest = project.nearest_names(from);
        return unknown(
            &format!("no name `{}` in this project", escaped(from)),
            &nearest,
            json,
        );
    }
    if index.declaration(to).is_none() {
        let nearest = project.nearest_names(to);
        return unknown(
            &format!("no name `{}` in this project", escaped(to)),
            &nearest,
            json,
        );
    }
    match query::move_member(&project.workspace, from, to) {
        Ok(rewrites) => write_out(
            &mut project,
            &rewrites,
            &[],
            &Whole::NONE,
            None,
            dry_run,
            json,
        ),
        Err(message) => unknown(&escaped(&message), &[], json),
    }
}

/// Whether the project declares `symbol` as a method, which is what the call
/// graph answers about.
fn method(project: &Project, symbol: &str) -> bool {
    project
        .workspace
        .index()
        .declaration(symbol)
        .is_some_and(|declared| declared.kind == DeclKind::Method)
}

/// The failure for a name that is not a method of the project.
fn unknown_method(project: &Project, symbol: &str, json: bool) -> ExitCode {
    let nearest: Vec<String> = query::nearest_methods(project.workspace.index(), symbol, NEAREST)
        .into_iter()
        .map(escaped)
        .collect();
    unknown(
        &format!("no method `{}` in this project", escaped(symbol)),
        &nearest,
        json,
    )
}

/// The failure for several names when one or more of them is not a method of
/// the project: each unknown name, in the order given, with its nearest
/// methods. The JSON gives `error` and `unknown`, a list of `symbol` and
/// `nearest`.
fn unknown_methods(project: &Project, symbols: &[&str], json: bool) -> ExitCode {
    let unknown: Vec<(String, Vec<String>)> = symbols
        .iter()
        .map(|symbol| {
            let nearest = query::nearest_methods(project.workspace.index(), symbol, NEAREST)
                .into_iter()
                .map(escaped)
                .collect();
            (escaped(symbol), nearest)
        })
        .collect();
    let names: Vec<String> = unknown
        .iter()
        .map(|(name, _)| format!("`{name}`"))
        .collect();
    let error = match names.as_slice() {
        [one] => format!("no method {one} in this project"),
        many => format!("no methods {} in this project", many.join(", ")),
    };
    if json {
        let records: Vec<Value> = unknown
            .iter()
            .map(|(symbol, nearest)| json!({ "symbol": symbol, "nearest": nearest }))
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "error": error,
                "unknown": records,
            }))
        );
        return ExitCode::FAILURE;
    }
    eprintln!("error: {error}");
    for (symbol, nearest) in &unknown {
        if nearest.is_empty() {
            eprintln!("nearest to {symbol}: none");
            continue;
        }
        eprintln!("nearest to {symbol}:");
        for name in nearest {
            eprintln!("  {name}");
        }
    }
    ExitCode::FAILURE
}

/// One method's answer: the JSON document without its `schemaVersion`, and
/// the text lines.
type Answer = (Map<String, Value>, Vec<String>);

/// Opens the project and prints `answer` for each name in `symbols`.
///
/// One name prints its answer as it is, and an unknown one fails with its
/// nearest methods. Several names are taken in the order given, each once.
/// Every name is checked before anything prints, and any unknown one fails
/// the whole call. Each answer then prints under a line with its name, its
/// lines indented by two spaces. The JSON is `schemaVersion` and `methods`,
/// the list of each name's document.
fn per_method(
    symbols: &[String],
    json: bool,
    answer: impl Fn(&mut Project, &str) -> Answer,
) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let mut names: Vec<&str> = Vec::new();
    for symbol in symbols {
        if !names.contains(&symbol.as_str()) {
            names.push(symbol);
        }
    }
    if let [symbol] = names.as_slice() {
        if !method(&project, symbol) {
            return unknown_method(&project, symbol, json);
        }
        let (document, lines) = answer(&mut project, symbol);
        if json {
            let mut out = Map::new();
            out.insert("schemaVersion".into(), Value::from(SCHEMA_VERSION));
            out.extend(document);
            print!("{}", pretty(Value::Object(out)));
        } else {
            for line in lines {
                println!("{line}");
            }
        }
        return ExitCode::SUCCESS;
    }
    let unknown: Vec<&str> = names
        .iter()
        .copied()
        .filter(|symbol| !method(&project, symbol))
        .collect();
    if !unknown.is_empty() {
        return unknown_methods(&project, &unknown, json);
    }
    let answers: Vec<(&str, Answer)> = names
        .iter()
        .map(|symbol| (*symbol, answer(&mut project, symbol)))
        .collect();
    if json {
        let methods: Vec<Value> = answers
            .into_iter()
            .map(|(_, (document, _))| Value::Object(document))
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "methods": methods,
            }))
        );
        return ExitCode::SUCCESS;
    }
    for (symbol, (_, lines)) in answers {
        println!("{}", escaped(symbol));
        for line in lines {
            println!("  {line}");
        }
    }
    ExitCode::SUCCESS
}

/// `nvs code calls`: the editor's call hierarchy of each method, the callers
/// and then the callees, each with the places it calls.
pub(crate) fn calls(symbols: &[String], json: bool) -> ExitCode {
    per_method(symbols, json, calls_of)
}

/// The callers and the callees of the method `symbol`, each with the places
/// it calls.
fn calls_of(project: &mut Project, symbol: &str) -> Answer {
    let owned = |found: Vec<nvs_lsp::calls::Calls<'_>>| -> Vec<(String, Vec<Site>)> {
        found
            .into_iter()
            .map(|calls| {
                let sites = calls.sites.into_iter().cloned().collect();
                (calls.method, sites)
            })
            .collect()
    };
    let index = project.workspace.index();
    let found: Vec<(&str, String, Vec<Site>)> = owned(nvs_lsp::calls::incoming(index, symbol))
        .into_iter()
        .map(|(method, sites)| ("caller", method, sites))
        .chain(
            owned(nvs_lsp::calls::outgoing(index, symbol))
                .into_iter()
                .map(|(method, sites)| ("callee", method, sites)),
        )
        .collect();
    let rows: Vec<(&str, String, Vec<Place>)> = found
        .iter()
        .map(|(side, method, sites)| {
            let places = sites.iter().map(|site| project.place(site)).collect();
            (*side, escaped(method), places)
        })
        .collect();
    let side = |wanted: &str| -> Vec<Value> {
        rows.iter()
            .filter(|(side, _, _)| *side == wanted)
            .map(|(_, method, places)| {
                let sites: Vec<Value> = places
                    .iter()
                    .map(|place| {
                        let mut record = place.json();
                        record.insert("text".into(), Value::from(place.source.clone()));
                        Value::Object(record)
                    })
                    .collect();
                json!({ "symbol": method, "sites": sites })
            })
            .collect()
    };
    let mut document = Map::new();
    document.insert("symbol".into(), Value::from(escaped(symbol)));
    document.insert("callers".into(), Value::from(side("caller")));
    document.insert("callees".into(), Value::from(side("callee")));
    let width = rows
        .iter()
        .flat_map(|(_, _, places)| places)
        .map(|place| place.position().len())
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    for (side, method, places) in &rows {
        lines.push(format!("{method}  {side}"));
        for place in places {
            lines.push(format!("  {:<width$}  {}", place.position(), place.source));
        }
    }
    (document, lines)
}

/// `nvs code reach`: the routes and the tests the used-by lens counts for
/// each method, read off the call graph.
pub(crate) fn reach(symbols: &[String], json: bool) -> ExitCode {
    per_method(symbols, json, reach_of)
}

/// The routes and the tests that reach the method `symbol`.
fn reach_of(project: &mut Project, symbol: &str) -> Answer {
    let found = nvs_lsp::reach::reach(project.workspace.index(), symbol);
    let routes: Vec<String> = found.routes.iter().map(|route| escaped(route)).collect();
    let tests: Vec<String> = found.tests.iter().map(|test| escaped(test)).collect();
    let mut lines: Vec<String> = routes
        .iter()
        .map(|route| format!("route  {route}"))
        .collect();
    lines.extend(tests.iter().map(|test| format!("test   {test}")));
    if found.at_least {
        lines.push(
            "at least: more routes or tests may reach this method. Some calls are only known when the program runs."
                .to_owned(),
        );
    }
    let mut document = Map::new();
    document.insert("symbol".into(), Value::from(escaped(symbol)));
    document.insert("routes".into(), Value::from(routes));
    document.insert("tests".into(), Value::from(tests));
    document.insert("atLeast".into(), Value::from(found.at_least));
    (document, lines)
}

/// `nvs code rename`: the editor's rename of a name or of the symbol at a
/// position, written to disk through [`write_out`], or printed with
/// `--dry-run`.
pub(crate) fn rename(target: &str, new_name: &str, dry_run: bool, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let renamed = if let Some((file, line, column)) = split_position(target) {
        let Some(path) = project.file(file) else {
            let nearest = project.nearest_files(file);
            return unknown(
                &format!("no file `{}` in this project", escaped(file)),
                &nearest,
                json,
            );
        };
        let at = query::Target::Position {
            path: &path,
            line,
            column,
        };
        query::rename(&project.workspace, at, new_name)
    } else {
        if project.workspace.index().declaration(target).is_none() {
            let nearest = project.nearest_names(target);
            return unknown(
                &format!("no name `{}` in this project", escaped(target)),
                &nearest,
                json,
            );
        }
        query::rename(&project.workspace, query::Target::Name(target), new_name)
    };
    match renamed {
        Ok(renamed) => {
            let moves: Vec<(PathBuf, PathBuf)> = renamed.file.into_iter().collect();
            write_out(
                &mut project,
                &renamed.rewrites,
                &moves,
                &Whole::NONE,
                None,
                dry_run,
                json,
            )
        }
        Err(message) => unknown(&escaped(&message), &[], json),
    }
}

/// What one line of a `--from` list renames: a name, or a file, a line and a
/// column.
enum Listed {
    /// A name in the index's form.
    Name(String),
    /// A file's key, a line and a column, each from 1.
    Position(PathBuf, u32, u32),
}

/// `nvs code rename --from`: every rename of a list, one `<target>
/// <new-name>` per line, applied as one plan ([`query::rename_all`]) and
/// written to disk through [`write_out`] in one write, or printed with
/// `--dry-run`. Each `--json` edit carries the line of the list it came
/// from as `fromLine`. A line that names nothing in the project fails the whole list.
pub(crate) fn rename_from(from: &str, dry_run: bool, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let list = match std::fs::read_to_string(project.cwd.join(from)) {
        Ok(list) => list,
        Err(err) => {
            return unknown(
                &format!("`{}` cannot be read: {err}", escaped(from)),
                &[],
                json,
            );
        }
    };
    let mut listed: Vec<(u32, Listed, String)> = Vec::new();
    for (at, line) in list.lines().enumerate() {
        let number = u32::try_from(at + 1).unwrap_or(u32::MAX);
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let [target, new_name] = parts[..] else {
            return unknown(
                &format!(
                    "line {number}: write what to rename and the new name, separated by a space"
                ),
                &[],
                json,
            );
        };
        let target = if let Some((file, line, column)) = split_position(target) {
            let Some(path) = project.file(file) else {
                let nearest = project.nearest_files(file);
                return unknown(
                    &format!("line {number}: no file `{}` in this project", escaped(file)),
                    &nearest,
                    json,
                );
            };
            Listed::Position(path, line, column)
        } else {
            if project.workspace.index().declaration(target).is_none() {
                let nearest = project.nearest_names(target);
                return unknown(
                    &format!(
                        "line {number}: no name `{}` in this project",
                        escaped(target)
                    ),
                    &nearest,
                    json,
                );
            }
            Listed::Name(target.to_owned())
        };
        listed.push((number, target, new_name.to_owned()));
    }
    if listed.is_empty() {
        return unknown(&format!("`{}` has no renames", escaped(from)), &[], json);
    }
    let labels: Vec<String> = listed
        .iter()
        .map(|(number, _, _)| format!("line {number}"))
        .collect();
    let targets: Vec<(query::Target<'_>, &str, &str)> = listed
        .iter()
        .zip(&labels)
        .map(|((_, target, new_name), label)| {
            let target = match target {
                Listed::Name(name) => query::Target::Name(name),
                Listed::Position(path, line, column) => query::Target::Position {
                    path,
                    line: *line,
                    column: *column,
                },
            };
            (target, new_name.as_str(), label.as_str())
        })
        .collect();
    let batch = match query::rename_all(&project.workspace, &targets) {
        Ok(batch) => batch,
        Err(message) => return unknown(&escaped(&message), &[], json),
    };
    let lines: BTreeMap<Rewrite, u32> = batch
        .rewrites
        .iter()
        .map(|(at, rewrite)| (rewrite.clone(), listed[*at].0))
        .collect();
    let rewrites: Vec<Rewrite> = batch.rewrites.into_iter().map(|(_, it)| it).collect();
    write_out(
        &mut project,
        &rewrites,
        &batch.moves,
        &Whole::NONE,
        Some(&lines),
        dry_run,
        json,
    )
}

/// `nvs code move`: a file move of a file or a directory, with every edit
/// [`rename::relocated`] gives, written to disk through [`write_out`], or
/// printed with `--dry-run`.
///
/// Both paths are judged before anything is computed: `from` exists and is
/// inside the project root, and `to` is inside it too, does not exist, is not
/// inside `from`, and has a directory that exists. A directory is not created
/// (the rule's Declined list). Each autoloaded type a moved file declares is
/// renamed to the name its new path gives, and an autoloaded file no root
/// reaches at its new path is refused. The edits and the move are checked
/// before anything is written ([`nvs_lsp::refactor::checked`]), and a move
/// that would add an error writes nothing and prints that error.
pub(crate) fn move_path(from: &str, to: &str, dry_run: bool, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let moves = match judged_move(&project, from, to) {
        Ok(moving) => vec![moving],
        Err((message, nearest)) => return unknown(&message, &nearest, json),
    };
    let workspace = &project.workspace;
    let rewrites = match rename::relocated(workspace.documents(), workspace.index(), &moves) {
        Ok(rewrites) => rewrites,
        Err(message) => return unknown(&escaped(&message), &[], json),
    };
    let rewrites = match nvs_lsp::refactor::checked(
        workspace.documents(),
        workspace.index(),
        rewrites,
        &moves,
    ) {
        Ok(checked) => checked.rewrites,
        Err(broken) => return unknown(&escaped(&broken.refusal()), &[], json),
    };
    write_out(
        &mut project,
        &rewrites,
        &moves,
        &Whole::NONE,
        None,
        dry_run,
        json,
    )
}

/// `from` and `to` as the keys a move is made under, or the message and the
/// nearest files of the first check that fails.
fn judged_move(
    project: &Project,
    from: &str,
    to: &str,
) -> Result<(PathBuf, PathBuf), (String, Vec<String>)> {
    let root = project.workspace.root();
    let refused = |message: String| Err((message, Vec::new()));
    let source = canonical_key(&project.cwd.join(from));
    if std::fs::symlink_metadata(&source).is_err() {
        return Err((
            format!("no file or directory `{}` in this project", escaped(from)),
            project.nearest_files(from),
        ));
    }
    if !inside(root, &source) {
        return refused(format!(
            "`{}` is outside the project, so nothing was changed.",
            escaped(from)
        ));
    }
    let written = project.cwd.join(to);
    let (Some(parent), Some(name)) = (written.parent(), written.file_name()) else {
        return refused(format!(
            "`{}` is not a path that a file can be moved to.",
            escaped(to)
        ));
    };
    if !parent.is_dir() {
        return refused(format!(
            "The directory `{}` does not exist. Create it, then run the command again.",
            project.shown(parent)
        ));
    }
    let target = canonical_key(parent).join(name);
    if !inside(root, &target) {
        return refused(format!(
            "`{}` is outside the project, so nothing was changed.",
            escaped(to)
        ));
    }
    if std::fs::symlink_metadata(&target).is_ok() {
        return refused(format!(
            "`{}` already exists, so nothing was changed.",
            project.shown(&target)
        ));
    }
    if target.starts_with(&source) {
        return refused(format!(
            "`{}` is inside `{}`. A directory cannot be moved into itself.",
            project.shown(&target),
            project.shown(&source)
        ));
    }
    Ok((source, target))
}

/// One class `order` names: where it is declared, and its full name.
struct Named {
    place: Place,
    symbol: String,
}

impl Named {
    fn new(project: &mut Project, path: &Path, class: &order::Class) -> Self {
        let site = Site {
            path: path.to_owned(),
            start: class.at,
            end: class.at,
        };
        Self {
            place: project.place(&site),
            symbol: escaped(&class.name),
        }
    }

    fn json(&self) -> Map<String, Value> {
        let mut record = self.place.json();
        record.insert("symbol".into(), Value::from(self.symbol.clone()));
        record
    }
}

/// What a note says about its class, in the text and in the JSON.
fn note_text(kind: order::NoteKind) -> (&'static str, &'static str) {
    match kind {
        order::NoteKind::KeyOrderChanges => (
            "keyOrderChanges",
            "note: the JSON key order of this class changes.",
        ),
        order::NoteKind::KeptAsWritten => (
            "keptAsWritten",
            "note: this class has `Core\\Json\\KeepKeyOrder`, and no member order keeps its \
             JSON keys. It is left as written.",
        ),
    }
}

/// `nvs code order`: [`order::of`] over each file `paths` names, or over
/// every file of the project, written through [`planned`] and [`committed`].
///
/// `--check` and `--dry-run` stop before the write. A file with a syntax
/// error, or one whose moved text does not format, is skipped with a line on
/// standard error, and the other files are still ordered
/// (`rule:tooling/class-members-have-one-order`).
pub(crate) fn order(paths: &[String], check: bool, dry_run: bool, json: bool) -> ExitCode {
    let mut project = match Project::open() {
        Ok(project) => project,
        Err(code) => return code,
    };
    let files = match ordered_files(&project, paths) {
        Ok(files) => files,
        Err((message, nearest)) => return unknown(&message, &nearest, json),
    };
    let mut rewrites: Vec<Rewrite> = Vec::new();
    let mut moved: Vec<(PathBuf, Vec<Named>)> = Vec::new();
    let mut notes: Vec<(Named, order::NoteKind)> = Vec::new();
    let mut skipped: Vec<(String, Option<Named>)> = Vec::new();
    for path in files {
        let text = match project.snapshot(&path) {
            Ok(text) => text.to_owned(),
            Err(message) => return unknown(&message, &[], json),
        };
        let mut map = SourceMap::new();
        let id = map.add(project.shown(&path), text.as_str());
        match order::of(map.file(id)) {
            Ok(ordered) => {
                for note in &ordered.notes {
                    notes.push((Named::new(&mut project, &path, &note.class), note.kind));
                }
                let Some(after) = ordered.text else { continue };
                let classes = ordered
                    .moved
                    .iter()
                    .map(|class| Named::new(&mut project, &path, class))
                    .collect();
                moved.push((path.clone(), classes));
                rewrites.push(Rewrite {
                    site: Site {
                        path,
                        start: 0,
                        end: BytePos::try_from(text.len()).unwrap_or(BytePos::MAX),
                    },
                    text: after,
                });
            }
            Err(order::Refused::DoesNotParse) => skipped.push((project.shown(&path), None)),
            Err(order::Refused::Unsafe(class)) => {
                let named = Named::new(&mut project, &path, &class);
                skipped.push((project.shown(&path), Some(named)));
            }
        }
    }
    let rewrites: Vec<&Rewrite> = rewrites.iter().collect();
    let changes = match planned(&mut project, &rewrites, &[]) {
        Ok(changes) => changes,
        Err(message) => return unknown(&message, &[], json),
    };
    let written = !check && !dry_run;
    if written && let Err(message) = committed(&project, &changes, &[], &[], &[]) {
        return unknown(&message, &[], json);
    }
    let status = if check && !moved.is_empty() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    };
    if json {
        let files: Vec<Value> = moved
            .iter()
            .map(|(path, classes)| {
                let classes: Vec<Value> = classes
                    .iter()
                    .map(|class| Value::Object(class.json()))
                    .collect();
                json!({ "path": project.shown(path), "classes": classes })
            })
            .collect();
        let notes: Vec<Value> = notes
            .iter()
            .map(|(class, kind)| {
                let mut record = class.json();
                record.insert("note".into(), Value::from(note_text(*kind).0));
                Value::Object(record)
            })
            .collect();
        let skipped: Vec<Value> = skipped
            .iter()
            .map(|(path, class)| {
                json!({
                    "path": path,
                    "reason": if class.is_some() { "doesNotFormat" } else { "syntaxError" },
                    "symbol": class.as_ref().map(|class| class.symbol.clone()),
                })
            })
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "files": files,
                "notes": notes,
                "skipped": skipped,
                "written": written,
            }))
        );
        return status;
    }
    for (path, class) in &skipped {
        match class {
            None => eprintln!("warning: `{path}` has a syntax error, so it was not ordered."),
            Some(class) => eprintln!(
                "warning: {}  {}: the moved members do not format, so `{path}` was not changed.",
                class.place.position(),
                class.symbol
            ),
        }
    }
    if written {
        for (path, classes) in &moved {
            let noun = if classes.len() == 1 {
                "class"
            } else {
                "classes"
            };
            println!("{}  {} {noun}", project.shown(path), classes.len());
        }
    } else {
        let width = moved
            .iter()
            .flat_map(|(_, classes)| classes)
            .map(|class| class.place.position().len())
            .max()
            .unwrap_or(0);
        for class in moved.iter().flat_map(|(_, classes)| classes) {
            println!("{:<width$}  {}", class.place.position(), class.symbol);
        }
    }
    if !check {
        for (class, kind) in &notes {
            println!(
                "{}  {}  {}",
                class.place.position(),
                class.symbol,
                note_text(*kind).1
            );
        }
    }
    status
}

/// The files `paths` names, each a file the project indexed or a directory
/// under the project root, sorted. No path is every file of the project.
fn ordered_files(
    project: &Project,
    paths: &[String],
) -> Result<Vec<PathBuf>, (String, Vec<String>)> {
    let index = project.workspace.index();
    let root = project.workspace.root();
    let mut files: Vec<PathBuf> = if paths.is_empty() {
        index.files().map(Path::to_owned).collect()
    } else {
        Vec::new()
    };
    for written in paths {
        let path = canonical_key(&project.cwd.join(written));
        if !inside(root, &path) {
            return Err((
                format!(
                    "`{}` is outside the project, so nothing was changed.",
                    escaped(written)
                ),
                Vec::new(),
            ));
        }
        if index.holds(&path) {
            files.push(path);
        } else if path.is_dir() {
            files.extend(
                index
                    .files()
                    .filter(|file| file.starts_with(&path))
                    .map(Path::to_owned),
            );
        } else {
            return Err((
                format!(
                    "no file or directory `{}` in this project",
                    escaped(written)
                ),
                project.nearest_files(written),
            ));
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// `path:line:column` split into the file, the line and the column, and
/// `None` for anything else, a name such as `Shop\Cart::MAX` included.
fn split_position(spec: &str) -> Option<(&str, u32, u32)> {
    let (rest, column) = spec.rsplit_once(':')?;
    let (file, line) = rest.rsplit_once(':')?;
    let line: u32 = line.parse().ok()?;
    let column: u32 = column.parse().ok()?;
    (!file.is_empty() && line > 0 && column > 0).then_some((file, line, column))
}

/// A line and a column, each from 1, the column in characters.
type LineColumn = (u32, u32);

/// `path:line:column-line:column` split into the file, the first line and
/// column, and the last line and column, each from 1. `None` for anything
/// else, and for a range whose end is before its start.
fn split_range(spec: &str) -> Option<(&str, LineColumn, LineColumn)> {
    let (first, last) = spec.rsplit_once('-')?;
    let (file, line, column) = split_position(first)?;
    let (end_line, end_column) = last.split_once(':')?;
    let end: (u32, u32) = (end_line.parse().ok()?, end_column.parse().ok()?);
    (end.0 > 0 && end.1 > 0 && (line, column) <= end).then_some((file, (line, column), end))
}

/// One file the write path changes: the bytes the edits were computed
/// against, the bytes it writes, and how many edits make the difference.
struct Change {
    path: PathBuf,
    before: String,
    after: String,
    edits: usize,
}

/// The whole files a write verb deletes or creates, beside its edits.
struct Whole<'a> {
    /// Each file the verb deletes.
    deleted: &'a [PathBuf],
    /// Each file the verb creates, with its text. Nothing may be at its path.
    created: &'a [(PathBuf, String)],
}

impl Whole<'_> {
    /// No file deleted or created.
    const NONE: Whole<'static> = Whole {
        deleted: &[],
        created: &[],
    };
}

/// The write path of `rename`, `move`, `delete`, `extract` and `implement`,
/// and what they print.
///
/// In this order: every edit is applied in memory, and a file that cannot be
/// read or an edit that does not fit its file writes nothing. Every path and
/// every move's target is checked to be inside the project root. Each file is
/// read again, and one whose bytes differ from the bytes the edits were
/// computed against writes nothing. Then each file of `whole.created` is
/// created, each file is written, each file of `whole.deleted` is removed,
/// and each move is made after all of them. `--dry-run` stops before the
/// second read.
///
/// The bytes the edits were computed against are each file's first read in
/// this call, which is right after the analysis. A crash between two writes
/// leaves the project half changed, and `nvs check` then names every site
/// that no longer resolves. A journal is not kept (the rule's Declined list).
///
/// `lines` is given for `rename --from`: the line of the list each edit came
/// from, which each `--json` edit carries as `fromLine`, since `line` is its
/// position's.
fn write_out(
    project: &mut Project,
    rewrites: &[Rewrite],
    moves: &[(PathBuf, PathBuf)],
    whole: &Whole<'_>,
    lines: Option<&BTreeMap<Rewrite, u32>>,
    dry_run: bool,
    json: bool,
) -> ExitCode {
    let deleted = whole.deleted;
    let created = whole.created;
    let mut rewrites: Vec<&Rewrite> = rewrites.iter().collect();
    rewrites.sort();
    rewrites.dedup();
    let changes = match planned(project, &rewrites, moves) {
        Ok(changes) => changes,
        Err(message) => return unknown(&message, &[], json),
    };
    let mut gone: Vec<Change> = Vec::with_capacity(deleted.len());
    for path in deleted {
        if !inside(project.workspace.root(), path) {
            let message = format!(
                "`{}` is outside the project, so nothing was changed.",
                project.shown(path)
            );
            return unknown(&message, &[], json);
        }
        match project.snapshot(path) {
            Ok(before) => gone.push(Change {
                path: path.clone(),
                before: before.to_owned(),
                after: String::new(),
                edits: 0,
            }),
            Err(message) => return unknown(&message, &[], json),
        }
    }
    let shown_deleted: Vec<String> = deleted.iter().map(|path| project.shown(path)).collect();
    for (path, _) in created {
        if !inside(project.workspace.root(), path) {
            let message = format!(
                "`{}` is outside the project, so nothing was changed.",
                project.shown(path)
            );
            return unknown(&message, &[], json);
        }
        if path.exists() {
            let message = format!(
                "`{}` already exists, so nothing was changed.",
                project.shown(path)
            );
            return unknown(&message, &[], json);
        }
    }
    let shown_created: Vec<String> = created
        .iter()
        .map(|(path, _)| project.shown(path))
        .collect();
    let edits: Vec<(Place, String, String)> = rewrites
        .iter()
        .map(|rewrite| {
            let place = project.place(&rewrite.site);
            let old = project
                .text(&rewrite.site.path)
                .get(rewrite.site.start as usize..rewrite.site.end as usize)
                .map(escaped)
                .unwrap_or_default();
            (place, old, escaped(&rewrite.text))
        })
        .collect();
    let shown_moves: Vec<(String, String)> = moves
        .iter()
        .map(|(from, to)| (project.shown(from), project.shown(to)))
        .collect();
    if !dry_run && let Err(message) = committed(project, &changes, moves, &gone, created) {
        return unknown(&message, &[], json);
    }
    if json {
        let edits: Vec<Value> = edits
            .iter()
            .zip(&rewrites)
            .map(|((place, old, new), rewrite)| {
                let mut record = place.json();
                record.insert("old".into(), Value::from(old.clone()));
                record.insert("new".into(), Value::from(new.clone()));
                if let Some(lines) = lines {
                    record.insert("fromLine".into(), Value::from(lines.get(*rewrite).copied()));
                }
                Value::Object(record)
            })
            .collect();
        let moved: Vec<Value> = shown_moves
            .iter()
            .map(|(from, to)| json!({ "from": from, "to": to }))
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "edits": edits,
                "moved": moved,
                "deleted": shown_deleted,
                "created": shown_created,
                "written": !dry_run,
            }))
        );
        return ExitCode::SUCCESS;
    }
    if dry_run {
        let width = edits
            .iter()
            .map(|(place, _, _)| place.position().len())
            .max()
            .unwrap_or(0);
        for (place, old, new) in &edits {
            println!("{:<width$}  {old} -> {new}", place.position());
        }
        for (from, to) in &shown_moves {
            println!("move {from} -> {to}");
        }
        for path in &shown_deleted {
            println!("delete {path}");
        }
        for path in &shown_created {
            println!("create {path}");
        }
        return ExitCode::SUCCESS;
    }
    for change in &changes {
        let noun = if change.edits == 1 { "edit" } else { "edits" };
        println!("{}  {} {noun}", project.shown(&change.path), change.edits);
    }
    for (from, to) in &shown_moves {
        println!("moved {from} -> {to}");
    }
    for path in &shown_deleted {
        println!("deleted {path}");
    }
    for path in &shown_created {
        println!("created {path}");
    }
    ExitCode::SUCCESS
}

/// Each changed file's bytes before and after `rewrites`, which are sorted,
/// once every path and every move is judged to be inside the project root.
fn planned(
    project: &mut Project,
    rewrites: &[&Rewrite],
    moves: &[(PathBuf, PathBuf)],
) -> Result<Vec<Change>, String> {
    let root = project.workspace.root().to_owned();
    let outside = |path: &Path| {
        format!(
            "`{}` is outside the project, so nothing was changed.",
            escaped(&relative(path, &root))
        )
    };
    let judged = rewrites
        .iter()
        .map(|rewrite| rewrite.site.path.as_path())
        .chain(
            moves
                .iter()
                .flat_map(|(from, to)| [from.as_path(), to.as_path()]),
        );
    for path in judged {
        if !inside(&root, path) {
            return Err(outside(path));
        }
    }
    let mut changes: Vec<Change> = Vec::new();
    for group in rewrites.chunk_by(|a, b| a.site.path == b.site.path) {
        let path = &group[0].site.path;
        let before = project.snapshot(path)?.to_owned();
        let after = applied(&before, group).ok_or_else(|| {
            format!(
                "An edit does not fit `{}`, so nothing was changed.",
                project.shown(path)
            )
        })?;
        changes.push(Change {
            path: path.clone(),
            before,
            after,
            edits: group.len(),
        });
    }
    Ok(changes)
}

/// Whether `path` is `root` or under it, with no `.` or `..` part that could
/// lead out of it. Every path the index holds is a canonical key, so this
/// fails only on a bug, and it fails closed.
fn inside(root: &Path, path: &Path) -> bool {
    path.starts_with(root)
        && !path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
}

/// `text` with each of `rewrites` applied, which are in source order, and
/// `None` when two overlap or one is not on a character boundary.
fn applied(text: &str, rewrites: &[&Rewrite]) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for rewrite in rewrites {
        out.push_str(text.get(at..rewrite.site.start as usize)?);
        out.push_str(&rewrite.text);
        at = rewrite.site.end as usize;
    }
    out.push_str(text.get(at..)?);
    Some(out)
}

/// The variable a test sets to a file that is changed after the edits are
/// computed and before they are written. Only a debug build reads it, so a
/// release binary has no such hook.
#[cfg(debug_assertions)]
const CHANGE_BEFORE_WRITE: &str = "NVS_TEST_CODE_CHANGE_BEFORE_WRITE";

/// Writes `changes` once no file of them or of `gone` differs from the bytes
/// they were computed against and nothing is at a path of `created`. Each
/// file of `created` is created first, then `changes` are written, then each
/// file of `gone` is removed, and then `moves` are made.
fn committed(
    project: &Project,
    changes: &[Change],
    moves: &[(PathBuf, PathBuf)],
    gone: &[Change],
    created: &[(PathBuf, String)],
) -> Result<(), String> {
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os(CHANGE_BEFORE_WRITE) {
        let mut bytes = std::fs::read(&path).unwrap_or_default();
        bytes.push(b'\n');
        let _ = std::fs::write(&path, bytes);
    }
    for change in changes.iter().chain(gone) {
        let now = std::fs::read(&change.path).unwrap_or_default();
        if now != change.before.as_bytes() {
            return Err(format!(
                "`{}` changed while the command ran, so nothing was changed. Run the command again.",
                project.shown(&change.path)
            ));
        }
    }
    let sources: BTreeSet<&PathBuf> = moves.iter().map(|(from, _)| from).collect();
    for (_, to) in moves {
        if to.exists() && !sources.contains(to) {
            return Err(format!(
                "`{}` already exists, so nothing was changed.",
                project.shown(to)
            ));
        }
    }
    for (path, _) in created {
        if path.exists() {
            return Err(format!(
                "`{}` already exists, so nothing was changed.",
                project.shown(path)
            ));
        }
    }
    let mut done: Vec<String> = Vec::new();
    let partly = |what: String, done: &[String]| {
        if done.is_empty() {
            what
        } else {
            format!(
                "{what} These files were already changed: {}.",
                done.join(", ")
            )
        }
    };
    for (path, text) in created {
        if let Err(what) = create_new(project, path, text) {
            return Err(partly(what, &done));
        }
        done.push(project.shown(path));
    }
    for change in changes {
        if let Err(err) = std::fs::write(&change.path, &change.after) {
            let what = format!(
                "`{}` cannot be written: {err}.",
                project.shown(&change.path)
            );
            return Err(partly(what, &done));
        }
        done.push(project.shown(&change.path));
    }
    for change in gone {
        if let Err(err) = std::fs::remove_file(&change.path) {
            let what = format!(
                "`{}` cannot be deleted: {err}.",
                project.shown(&change.path)
            );
            return Err(partly(what, &done));
        }
        done.push(project.shown(&change.path));
    }
    // Two files that swap paths are each moved aside first, so that no move
    // replaces a file another move has not moved yet.
    let mut staged: Vec<(PathBuf, &PathBuf)> = Vec::with_capacity(moves.len());
    let swapping = moves.iter().any(|(_, to)| sources.contains(to));
    for (at, (from, to)) in moves.iter().enumerate() {
        if !swapping {
            staged.push((from.clone(), to));
            continue;
        }
        let name = from.file_name().unwrap_or_default().to_string_lossy();
        let aside = from.with_file_name(format!("{name}.moving-{at}"));
        if aside.exists() {
            let what = format!("`{}` already exists.", project.shown(&aside));
            return Err(partly(what, &done));
        }
        if let Err(err) = std::fs::rename(from, &aside) {
            let what = format!("`{}` cannot be moved: {err}.", project.shown(from));
            return Err(partly(what, &done));
        }
        staged.push((aside, to));
    }
    for (from, to) in staged {
        let (from, to) = (&from, to);
        // A type moved to another namespace may move into a directory that
        // does not exist yet. Each one is judged inside the root before it is
        // created. `nvs code move` refuses a missing directory before this.
        if let Some(parent) = to.parent()
            && !parent.is_dir()
        {
            if !inside(project.workspace.root(), parent) {
                let what = format!(
                    "`{}` is outside the project, so it was not created.",
                    project.shown(parent)
                );
                return Err(partly(what, &done));
            }
            if let Err(err) = std::fs::create_dir_all(parent) {
                let what = format!("`{}` cannot be created: {err}.", project.shown(parent));
                return Err(partly(what, &done));
            }
        }
        if let Err(err) = std::fs::rename(from, to) {
            let what = format!(
                "`{}` cannot be moved to `{}`: {err}.",
                project.shown(from),
                project.shown(to)
            );
            return Err(partly(what, &done));
        }
    }
    Ok(())
}

/// Creates the file `path` with `text`, and each missing directory above it,
/// each judged inside the project root first. A file already at `path` is an
/// error, since it is opened with `create_new`.
fn create_new(project: &Project, path: &Path, text: &str) -> Result<(), String> {
    use std::io::Write as _;
    if let Some(parent) = path.parent()
        && !parent.is_dir()
    {
        if !inside(project.workspace.root(), parent) {
            return Err(format!(
                "`{}` is outside the project, so it was not created.",
                project.shown(parent)
            ));
        }
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("`{}` cannot be created: {err}.", project.shown(parent)))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|err| format!("`{}` cannot be created: {err}.", project.shown(path)))?;
    file.write_all(text.as_bytes())
        .map_err(|err| format!("`{}` cannot be written: {err}.", project.shown(path)))
}

/// `file:10` or `file:10-20` split into the file and its lines, and the whole
/// file where no line range is written.
fn split_lines(spec: &str) -> (&str, RangeInclusive<u32>) {
    let whole = (spec, 1..=u32::MAX);
    let Some((file, lines)) = spec.rsplit_once(':') else {
        return whole;
    };
    let range = match lines.split_once('-') {
        Some((first, last)) => first.parse().ok().zip(last.parse().ok()),
        None => lines.parse().ok().map(|line| (line, line)),
    };
    range.map_or(whole, |(first, last)| (file, first..=last))
}

/// Whether `symbol` names something even though the project has no answer for
/// it: a `Core` name the project does not use.
fn known(project: &Project, symbol: &str) -> bool {
    project.workspace.index().declaration(symbol).is_some() || crate::agent::has_card(symbol)
}

/// The failure for a name or a file the project does not have.
fn unknown(error: &str, nearest: &[String], json: bool) -> ExitCode {
    if json {
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "error": error,
                "nearest": nearest,
            }))
        );
        return ExitCode::FAILURE;
    }
    eprintln!("error: {error}");
    if !nearest.is_empty() {
        eprintln!("nearest:");
        for name in nearest {
            eprintln!("  {name}");
        }
    }
    ExitCode::FAILURE
}

/// The word a kind is printed as: `method`, `enum case`.
fn kind(kind: DeclKind) -> &'static str {
    let described = kind.describe();
    described
        .split_once(' ')
        .map_or(described, |(_, noun)| noun)
}

/// The signature `nvs meta` writes for a method or a property, read from a
/// fresh parse of the file that declares it, and `None` for anything else.
fn signature(workspace: &Workspace, declared: &Declaration) -> Option<String> {
    if !matches!(declared.kind, DeclKind::Method | DeclKind::Property) {
        return None;
    }
    let analysed = nvs_lsp::analyse_file(workspace.documents(), &declared.site.path, 0)?;
    let loaded = analysed.loaded.iter().find(|loaded| {
        analysed
            .map
            .file(loaded.id)
            .path()
            .is_some_and(|path| canonical_key(path) == declared.site.path)
    })?;
    member_signature(&loaded.stmts, analysed.map.file(loaded.id), &declared.site)
}

/// The signature of the member whose name `site` covers, in `stmts`.
fn member_signature(stmts: &[Stmt], src: &SourceFile, site: &Site) -> Option<String> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::NamespaceDecl(decl) => decl
            .body
            .as_ref()
            .and_then(|block| member_signature(&block.stmts, src, site)),
        StmtKind::ClassDecl(decl) => in_members(&decl.members, src, site),
        StmtKind::InterfaceDecl(decl) => in_members(&decl.members, src, site),
        StmtKind::EnumDecl(decl) => in_members(&decl.members, src, site),
        _ => None,
    })
}

/// [`member_signature`] over one body, an enum inside it included.
fn in_members(members: &[ClassMember], src: &SourceFile, site: &Site) -> Option<String> {
    let covers = |span: nvs_diagnostics::Span| span.start < site.end && site.start < span.end;
    members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(method) if covers(method.name) => {
            Some(crate::meta::method_signature(method, src))
        }
        ClassMemberKind::Property(property) if covers(property.name) => {
            Some(crate::meta::property_signature(property, src))
        }
        ClassMemberKind::Enum(decl) => in_members(&decl.members, src, site),
        _ => None,
    })
}

/// The project root: the directory of the nearest `nvs.toml` at or above
/// `cwd`, and `cwd` itself when there is none.
fn project_root(cwd: &Path) -> PathBuf {
    cwd.ancestors()
        .find(|dir| dir.join("nvs.toml").is_file())
        .unwrap_or(cwd)
        .to_owned()
}

/// `path` relative to `cwd`, with `..` for each level above it and `/`
/// between the parts. Both are canonical keys.
fn relative(path: &Path, cwd: &Path) -> String {
    let path_parts: Vec<Component<'_>> = path.components().collect();
    let cwd_parts: Vec<Component<'_>> = cwd.components().collect();
    let shared = path_parts
        .iter()
        .zip(&cwd_parts)
        .take_while(|(a, b)| a == b)
        .count();
    if shared == 0 {
        return path.to_string_lossy().into_owned();
    }
    let mut parts: Vec<String> = vec!["..".to_owned(); cwd_parts.len() - shared];
    parts.extend(
        path_parts[shared..]
            .iter()
            .map(|part| part.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}

/// `text` with every control character but tab written as `\u{..}`.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_control() && ch != '\t' {
            let _ = write!(out, "\\u{{{:x}}}", u32::from(ch));
        } else {
            out.push(ch);
        }
    }
    out
}

/// [`escaped`] over each line of a doc comment, the line breaks kept.
fn escaped_lines(text: &str) -> String {
    text.split('\n').map(escaped).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_range_is_split_off_the_file_and_a_drive_letter_is_not() {
        assert_eq!(split_lines("src/Cart.nvs:3-9"), ("src/Cart.nvs", 3..=9));
        assert_eq!(split_lines("src/Cart.nvs:4"), ("src/Cart.nvs", 4..=4));
        assert_eq!(split_lines("src/Cart.nvs"), ("src/Cart.nvs", 1..=u32::MAX));
        assert_eq!(
            split_lines(r"C:\src\Cart.nvs"),
            (r"C:\src\Cart.nvs", 1..=u32::MAX)
        );
    }

    #[test]
    fn a_control_character_is_escaped_and_a_tab_is_not() {
        assert_eq!(escaped("a\u{1b}[31m\tb\r"), "a\\u{1b}[31m\tb\\u{d}");
    }
}
