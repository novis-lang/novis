//! The refactorings that need text from the person who runs them, reached
//! through `workspace/executeCommand` (`rule:ide/the-request-set-is-closed`).
//!
//! A code action carries one of [`COMMANDS`] with what the input box starts
//! from. The client shows its own input box, prefilled with that text, and
//! sends the command back with what the person typed. [`execute`] computes the
//! edit with the same engine `nvs code` calls, so the two clients never
//! disagree, and the server sends it with `workspace/applyEdit`. A refusal is
//! the engine's own sentence, and the server answers it as an error the client
//! shows.
//!
//! # Decision: one command id, read by the server and by the client
//!
//! The server declares each id in `executeCommandProvider`, and
//! `vscode-languageclient` registers it as an editor command. The extension
//! asks for the text in its `executeCommand` middleware, before the request is
//! sent, so no id is registered twice. A client without that step sends the
//! text the action carried, which changes nothing, and is answered so.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. The offer reads the member
//! under the cursor in the open document's tree and the index, with no
//! analysis. A command costs what its engine costs (`crate::signature_change`,
//! `crate::move_member`, `crate::extract_interface`).
//!
//! # Decision: the name *Extract interface…* starts from
//!
//! The class's own name followed by `Contract`, as `CartContract` for `Cart`.
//! Novis has no naming convention for an interface, and the class's own name
//! is taken. The text is only where the input box starts: the person types
//! the name they want.

use std::path::PathBuf;

use nvs_diagnostics::BytePos;
use serde_json::Value;

use crate::document::{Analysed, Documents};
use crate::index::{DeclKind, SymbolIndex};
use crate::rename::Rewrite;

/// *Change parameters…*: `[symbol, list]`, the method as `Class::method` and
/// its whole new parameter list as it is written in source.
pub const CHANGE_PARAMETERS: &str = "nvs.changeParameters";

/// *Move member…*: `[symbol, class]`, the member as `Class::member` and the
/// class it moves to.
pub const MOVE_MEMBER: &str = "nvs.moveMember";

/// *Extract interface…*: `[class, name]`, the class and the new interface's
/// name, with the default methods (`crate::extract_interface`).
pub const EXTRACT_INTERFACE: &str = "nvs.extractInterface";

/// Every command the server declares in `executeCommandProvider`.
pub const COMMANDS: &[&str] = &[CHANGE_PARAMETERS, MOVE_MEMBER, EXTRACT_INTERFACE];

/// The method the constructor is, which [`at`] does not offer: the engine
/// refuses it (`crate::signature_change`).
const CONSTRUCTOR: &str = "constructor";

/// One code action that runs a command: its title, its kind, the command and
/// the text each argument starts as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// What the client shows for it.
    pub title: String,
    /// The kind it is filed under.
    pub kind: &'static str,
    /// One of [`COMMANDS`].
    pub command: &'static str,
    /// The arguments, each a string.
    pub arguments: Vec<String>,
}

/// What a command writes: its edits, and the file it creates, with its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Executed {
    /// The edits to existing files.
    pub rewrites: Vec<Rewrite>,
    /// The file the command creates and its text, which only
    /// [`EXTRACT_INTERFACE`] has.
    pub created: Option<(PathBuf, String)>,
}

/// The commands offered at `offset` of the entry document: *Change
/// parameters…* on the name of a project method that is not a constructor,
/// carrying the method and its current parameter list, *Move member…* on the
/// name of a member of a project class that is not a constructor, carrying
/// the member and its class, and *Extract interface…* on the name of a
/// project class, carrying the class and the name the input box starts from.
#[must_use]
pub fn at(analysed: &Analysed, index: &SymbolIndex, offset: BytePos) -> Vec<Offer> {
    let Some((symbol, _)) = crate::index::declared_at(analysed, offset) else {
        return Vec::new();
    };
    let Some((class, name)) = symbol.rsplit_once("::") else {
        return extract_interface(index, &symbol).into_iter().collect();
    };
    let Some(declared) = index.declaration(&symbol) else {
        return Vec::new();
    };
    if name == CONSTRUCTOR || class.is_empty() {
        return Vec::new();
    }
    let movable = matches!(
        declared.kind,
        DeclKind::Method
            | DeclKind::Property
            | DeclKind::Const
            | DeclKind::TypeAlias
            | DeclKind::Enum
    ) && index
        .declaration(class)
        .is_some_and(|owner| owner.kind == DeclKind::Class);
    let mut offers = Vec::new();
    if declared.kind == DeclKind::Method
        && let Some(offer) = change_parameters(analysed, offset, &symbol, name)
    {
        offers.push(offer);
    }
    if movable {
        offers.push(Offer {
            title: format!("Move `{}` to another class…", name.trim_start_matches('$')),
            kind: crate::split_join::KIND,
            command: MOVE_MEMBER,
            arguments: vec![symbol.clone(), class.to_owned()],
        });
    }
    offers
}

/// *Extract interface…* on the class `symbol`, starting from its short name
/// followed by `Contract`.
fn extract_interface(index: &SymbolIndex, symbol: &str) -> Option<Offer> {
    let declared = index.declaration(symbol)?;
    if declared.kind != DeclKind::Class {
        return None;
    }
    let short = symbol.rsplit_once('\\').map_or(symbol, |(_, short)| short);
    Some(Offer {
        title: format!("Extract an interface from `{short}`…"),
        kind: crate::split_join::KIND,
        command: EXTRACT_INTERFACE,
        arguments: vec![symbol.to_owned(), format!("{short}Contract")],
    })
}

/// *Change parameters…* for the method `symbol` whose name covers `offset`.
fn change_parameters(
    analysed: &Analysed,
    offset: BytePos,
    symbol: &str,
    name: &str,
) -> Option<Offer> {
    let stmts = crate::imports::entry_stmts(analysed)?;
    let (_, method) = stmts_method(stmts, analysed, offset)?;
    let source = analysed.map.file(analysed.entry).text();
    let list = match (method.params.first(), method.params.last()) {
        (Some(first), Some(last)) => source
            .get(first.span.start as usize..last.span.end as usize)
            .unwrap_or_default()
            .to_owned(),
        _ => String::new(),
    };
    Some(Offer {
        title: format!("Change parameters of `{name}`…"),
        kind: crate::split_join::KIND,
        command: CHANGE_PARAMETERS,
        arguments: vec![symbol.to_owned(), list],
    })
}

/// The method whose name covers `offset`, found from its name's start.
fn stmts_method<'a>(
    stmts: &'a [nvs_syntax::ast::Stmt],
    analysed: &Analysed,
    offset: BytePos,
) -> Option<(nvs_diagnostics::Span, &'a nvs_syntax::ast::MethodMember)> {
    let source = analysed.map.file(analysed.entry).text();
    let bytes = source.as_bytes();
    let mut start = offset as usize;
    while start > 0
        && bytes
            .get(start - 1)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        start -= 1;
    }
    crate::inline::method_member(stmts, BytePos::try_from(start).ok()?)
}

/// What `command` writes with `arguments`, or why it writes nothing.
///
/// # Errors
///
/// Returns the sentence the client shows: an unknown command, arguments that
/// are not the command's, or the engine's refusal.
pub fn execute(
    documents: &Documents,
    index: &SymbolIndex,
    command: &str,
    arguments: &[Value],
) -> Result<Executed, String> {
    let edits = |rewrites: Vec<Rewrite>| Executed {
        rewrites,
        created: None,
    };
    let texts: Option<Vec<&str>> = arguments.iter().map(Value::as_str).collect();
    match command {
        CHANGE_PARAMETERS => {
            let Some([symbol, list]) = texts.as_deref() else {
                return Err(format!(
                    "`{CHANGE_PARAMETERS}` needs two strings: the method and its new parameter \
                     list."
                ));
            };
            let rewrites = crate::signature_change::signature(documents, index, symbol, list, &[])?;
            if rewrites.is_empty() {
                return Err(format!("The parameters of `{symbol}` did not change."));
            }
            Ok(edits(rewrites))
        }
        MOVE_MEMBER => {
            let Some([symbol, class]) = texts.as_deref() else {
                return Err(format!(
                    "`{MOVE_MEMBER}` needs two strings: the member and the class it moves to."
                ));
            };
            crate::move_member::moved_member(documents, index, symbol, class.trim()).map(edits)
        }
        EXTRACT_INTERFACE => {
            let Some([class, name]) = texts.as_deref() else {
                return Err(format!(
                    "`{EXTRACT_INTERFACE}` needs two strings: the class and the name of the \
                     interface."
                ));
            };
            let extracted =
                crate::extract_interface::extracted(documents, index, class, name, None)?;
            Ok(Executed {
                rewrites: extracted.rewrites,
                created: Some((extracted.path, extracted.text)),
            })
        }
        _ => Err(format!("`{command}` is not a command of this server.")),
    }
}
