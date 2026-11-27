//! `Core\Imap` — reads a mailbox an operator named in an `[imap.<name>]` block
//! (`rule:core-classes/imap`).
//!
//! `rule:core-api/tier-roster` places the class Native by test 3, as `Core\Mail`'s transport is,
//! because it waits on the outside world. A program names the block and nothing else, and
//! `[capabilities.imap] connect` grants it by name, so no value a request carries can choose a
//! host, a port or a credential. The name is a [`Qual::Sink`] for `Core\Mail`'s reason: a
//! `tainted` block name is input choosing between deployments.
//!
//! [`session`] is the protocol: the connection, TLS, the sign-in and each command, written and
//! read by `imap-codec`. This module reads the block, asks the grant, fetches an `xoauth2` token
//! through [`crate::mail::oauth`], and gives the program three classes:
//!
//! - `Core\Imap::open(string $endpoint): Core\Imap\Mailbox` opens one signed-in session.
//! - `Mailbox` lists the folders and selects one with `folder`.
//! - `Folder` has its `UIDVALIDITY`, searches and fetches a parsed `Core\Mail\Message`, and moves,
//!   flags and deletes a message by its UID.
//!
//! **A `directory` block is a mailbox on disk.** [`directory`] reads and changes it with the same
//! members, under `fs.read` for the block's directory when it opens and `fs.write` before each
//! change. A test and local work use it with no server.
//!
//! **A flag is checked before it is sent.** `flag` and `unflag` take `\Answered`, `\Flagged`,
//! `\Draft`, `\Seen` or a keyword such as `$Important`. `\Deleted` is refused, so only `delete`
//! sets it, and `delete` always expunges the message at once. `delete` and a `move` without `MOVE`
//! run `UID EXPUNGE` of the one UID, and a server without `UIDPLUS` cannot do either.
//!
//! **One session per `open`, closed with the request.** The session is filed with
//! [`Ctx::hold_open_connection`] under no memo and no lease, so it is never pooled, and the
//! request's teardown drops it. [`Held`]'s `Drop` sends `LOGOUT` then, with a deadline that has
//! already passed, so a teardown never waits on the server. There is no pool: a script polls, and
//! a session holds the folder it selected.
//!
//! **A session has one selected folder.** Each `Folder` call selects its own folder again when
//! another one was selected since, and a `UIDVALIDITY` that changed in between is an `IOError`,
//! because every UID the program holds for that folder now names another message.
//!
//! **What it spends:** one socket, one TLS session and one read buffer per `open`, freed when the
//! request ends. `fetch` holds the raw message and its parsed copy, about twice its size, and
//! refuses a message larger than the request's remaining memory before its body is read.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use nvs_config::capability::Scope;
use nvs_config::mail::Auth;
use nvs_runtime::{Ctx, Fault, NvsArray, Tag, ThrownClass, Value};
use nvs_syntax::duration;

use crate::mail::{configured_in, oauth};
use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
    Qual, TextOrigin,
};

mod directory;
pub(crate) mod session;

use directory::Directory;
use session::{Filter, Server, Session, SignIn};

/// The class name, as a refusal spells it.
pub(crate) const NAME: &str = r"Core\Imap";

/// `Core\Imap\Mailbox`.
pub(crate) const MAILBOX_NAME: &str = r"Core\Imap\Mailbox";

/// `Core\Imap\Folder`.
pub(crate) const FOLDER_NAME: &str = r"Core\Imap\Folder";

/// `Core\Imap::open`, as its errors spell it.
const OPEN: &str = r"Core\Imap::open";

/// The longest flag `flag` and `unflag` take, in bytes. A server keeps every keyword of a
/// folder, and a directory block writes each into a file with a ceiling, so one keyword is
/// bounded.
pub(super) const MAX_KEYWORD_BYTES: usize = 255;

/// The slot of [`MAILBOX`] and [`FOLDER`] that holds the session's key.
const HANDLE: usize = 0;

/// The slot of [`MAILBOX`] and [`FOLDER`] that holds the block's name.
const BLOCK: usize = 1;

/// The slot of [`FOLDER`] that holds the folder's name.
const FOLDER: usize = 2;

/// The slot of [`FOLDER`] that holds its `UIDVALIDITY`.
const VALIDITY: usize = 3;

/// `Core\Imap` — one static member.
// rank: 890
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[CoreMethod {
        name: "open",
        names: &["endpoint"],
        params: &[CoreTy::Text(Qual::Sink)],
        defaults: &[],
        return_ty: CoreTy::Instance(MAILBOX_NAME),
        text: TextOrigin::Unmarked,
        symbol: "nvs_core_imap_open",
        doc: Some(&OPEN_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Imap\Mailbox` — one open session.
// rank: 891
pub(crate) const MAILBOX: CoreClass = CoreClass {
    name: MAILBOX_NAME,
    doc: Some(&MAILBOX_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "folders",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            text: TextOrigin::Source,
            symbol: "nvs_core_imap_mailbox_folders",
            doc: Some(&FOLDERS_DOC),
        },
        CoreMethod {
            name: "folder",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(FOLDER_NAME),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_imap_mailbox_folder",
            doc: Some(&FOLDER_DOC),
        },
    ],
    slots: &["handle", "block"],
    constants: &[],
};

/// The options of `Folder::search`, in the order the bag flattens to.
const SEARCH_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "unseen",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "since",
        ty: CoreTy::Instance(crate::time::DATE_NAME),
        default: Const::Null,
    },
    CoreOption {
        name: "before",
        ty: CoreTy::Instance(crate::time::DATE_NAME),
        default: Const::Null,
    },
    CoreOption {
        name: "from",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "subject",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "limit",
        ty: CoreTy::Int,
        default: Const::Null,
    },
];

/// The ABI argument of each search option, after the receiver.
const UNSEEN: usize = 1;
/// See [`UNSEEN`].
const SINCE: usize = 2;
/// See [`UNSEEN`].
const BEFORE: usize = 3;
/// See [`UNSEEN`].
const FROM: usize = 4;
/// See [`UNSEEN`].
const SUBJECT: usize = 5;
/// See [`UNSEEN`].
const LIMIT: usize = 6;

/// `Core\Imap\Folder` — one folder of an open session.
// rank: 892
pub(crate) const FOLDER_CLASS: CoreClass = CoreClass {
    name: FOLDER_NAME,
    doc: Some(&FOLDER_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "uidValidity",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_imap_folder_uid_validity",
            doc: Some(&UID_VALIDITY_DOC),
        },
        CoreMethod {
            name: "search",
            names: &[],
            params: &[CoreTy::Options(SEARCH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Int),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_imap_folder_search",
            doc: Some(&SEARCH_DOC),
        },
        CoreMethod {
            name: "fetch",
            names: &["uid"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(crate::mail::receive::MESSAGE_NAME),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_imap_folder_fetch",
            doc: Some(&FETCH_DOC),
        },
        CoreMethod {
            name: "move",
            names: &["uid", "folder"],
            params: &[CoreTy::Int, CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_imap_folder_move",
            doc: Some(&MOVE_DOC),
        },
        CoreMethod {
            name: "markSeen",
            names: &["uid"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Void,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_imap_folder_mark_seen",
            doc: Some(&MARK_SEEN_DOC),
        },
        CoreMethod {
            name: "markUnseen",
            names: &["uid"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Void,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_imap_folder_mark_unseen",
            doc: Some(&MARK_UNSEEN_DOC),
        },
        CoreMethod {
            name: "flag",
            names: &["uid", "flag"],
            params: &[CoreTy::Int, CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_imap_folder_flag",
            doc: Some(&FLAG_DOC),
        },
        CoreMethod {
            name: "unflag",
            names: &["uid", "flag"],
            params: &[CoreTy::Int, CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_imap_folder_unflag",
            doc: Some(&UNFLAG_DOC),
        },
        CoreMethod {
            name: "delete",
            names: &["uid"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Void,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_imap_folder_delete",
            doc: Some(&DELETE_DOC),
        },
    ],
    slots: &["handle", "block", "name", "uidValidity"],
    constants: &[],
};

/// The class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Reads a mailbox over IMAP. The server and the sign-in are set in an `[imap.<name>]` \
            block of `nvs.toml`, and your program names the block.",
};

/// `open`'s card.
const OPEN_DOC: MethodDoc = MethodDoc {
    short: "Connects to the mailbox of the block `[imap.<endpoint>]` and signs in. The \
            connection is closed when the request or the script ends.",
    params: &[ParamDoc {
        name: "endpoint",
        desc: "The name of the block, such as `inbox` for `[imap.inbox]`. It must be listed \
               in `[capabilities.imap] connect`.",
        shape: &[],
    }],
    ret: "The open mailbox.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The name is not listed in `[capabilities.imap] connect`, no block has that \
                   name, or the block has no sign-in.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The server cannot be reached, its certificate is not trusted, or it does \
                   not accept the sign-in.",
        },
    ],
};

/// `Mailbox`'s card.
const MAILBOX_CARD: ClassDoc = ClassDoc {
    short: "An open mailbox that `Core\\Imap::open` returns.",
};

/// `folders`' card.
const FOLDERS_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of every folder you can open, such as `INBOX` and `Sent`.",
    params: &[],
    ret: "The names, in the order the server sends them. A `directory` block returns them in \
          alphabetical order. The server writes the names, so they are `tainted` (text from \
          outside your program).",
    errors: &[ErrorDoc {
        error: "IOError",
        desc: "The server does not answer.",
    }],
};

/// `folder`'s card.
const FOLDER_DOC: MethodDoc = MethodDoc {
    short: "Opens the folder `$name` of the mailbox.",
    params: &[ParamDoc {
        name: "name",
        desc: "The folder name, such as `INBOX`, as `folders` returns it.",
        shape: &[],
    }],
    ret: "The folder.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The block is a `directory` block, and the name is not allowed there. Such a \
                   block is a folder on disk that works like a mailbox. A name is not allowed \
                   when it is empty, `.` or `..`, or has a `/`, `\\`, `:` or control character. \
                   A name that ends with a dot or a space is not allowed either.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The folder does not exist, or the server does not answer.",
        },
    ],
};

/// `Folder`'s card.
const FOLDER_CARD: ClassDoc = ClassDoc {
    short: "One folder of an open mailbox. Each message in it has a UID (the number the server \
            gives each message in a folder).",
};

/// `uidValidity`'s card.
const UID_VALIDITY_DOC: MethodDoc = MethodDoc {
    short: "Returns the folder's `UIDVALIDITY` (a number that changes when the server numbers \
            the messages again). A UID is the number the server gives each message in a folder. \
            When the `UIDVALIDITY` changes, a UID you saved may now belong to another message.",
    params: &[],
    ret: "A number of 1 or more.",
    errors: &[],
};

/// `search`'s card.
const SEARCH_DOC: MethodDoc = MethodDoc {
    short: "Returns the UID (the number the server gives each message in a folder) of each \
            message that matches every option you set. With no option, it returns every message.",
    params: &[
        ParamDoc {
            name: "unseen",
            desc: "`true` finds only the messages that are not read yet.",
            shape: &[],
        },
        ParamDoc {
            name: "since",
            desc: "Finds the messages received on this day or later.",
            shape: &[],
        },
        ParamDoc {
            name: "before",
            desc: "Finds the messages received before this day. The day itself is not \
                   included.",
            shape: &[],
        },
        ParamDoc {
            name: "from",
            desc: "Finds the messages whose `From` header contains this text.",
            shape: &[],
        },
        ParamDoc {
            name: "subject",
            desc: "Finds the messages whose subject contains this text.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "Returns at most this many UIDs, the oldest first.",
            shape: &[],
        },
    ],
    ret: "The UIDs, oldest first.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`limit` is less than 1.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The server does not answer, or the folder's `UIDVALIDITY` changed. That is a \
                   number that changes when the server numbers the messages again.",
        },
    ],
};

/// `fetch`'s card.
const FETCH_DOC: MethodDoc = MethodDoc {
    short: "Reads the message `$uid` and parses it. The message is not marked as read.",
    params: &[ParamDoc {
        name: "uid",
        desc: "A UID (the number the server gives each message in a folder) that `search` \
               returned.",
        shape: &[],
    }],
    ret: "The parsed message. Its `uid()` is `$uid`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$uid` is less than 1 or larger than 4294967295.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "No message has that UID, or the server does not answer. It is also thrown \
                   when the message is larger than the memory the request has left. It is also \
                   thrown when the folder's `UIDVALIDITY` changed. That is a number that changes \
                   when the server numbers the messages again.",
        },
    ],
};

/// The `uid` parameter of the six members that change a message.
const UID_PARAM: ParamDoc = ParamDoc {
    name: "uid",
    desc: "A UID (the number the server gives each message in a folder) that `search` returned.",
    shape: &[],
};

/// The `RuntimeError` of a UID that is not 1 to 4294967295.
const UID_ERROR: ErrorDoc = ErrorDoc {
    error: "RuntimeError",
    desc: "`$uid` is less than 1 or larger than 4294967295.",
};

/// The `IOError` of every member that changes a message.
const CHANGE_ERROR: ErrorDoc = ErrorDoc {
    error: "IOError",
    desc: "No message has that UID, or the server does not answer. It is also thrown when the \
           folder's `UIDVALIDITY` changed. That is a number that changes when the server numbers \
           the messages again.",
};

/// The `flag` parameter of `flag` and `unflag`.
const FLAG_PARAM: ParamDoc = ParamDoc {
    name: "flag",
    desc: "`\\Answered`, `\\Flagged`, `\\Draft`, `\\Seen`, or a keyword such as `$Important`. \
           A keyword cannot have a space or any of these characters: `(){%*\"\\]`.",
    shape: &[],
};

/// The `RuntimeError` of a flag that cannot be set.
const FLAG_ERROR: ErrorDoc = ErrorDoc {
    error: "RuntimeError",
    desc: "`$flag` starts with `\\` and is not one of the four standard flags, or it has a space \
           or one of the characters `(){%*\"\\]`, or it is longer than 255 bytes. `\\Deleted` \
           throws too. To delete a message, call `delete`.",
};

/// `move`'s card.
const MOVE_DOC: MethodDoc = MethodDoc {
    short: "Moves the message `$uid` to the folder `$folder`. In that folder, the message has a \
            new UID.",
    params: &[
        UID_PARAM,
        ParamDoc {
            name: "folder",
            desc: "The name of the folder to move it to, such as `Archive`.",
            shape: &[],
        },
    ],
    ret: "Nothing.",
    errors: &[
        UID_ERROR,
        ErrorDoc {
            error: "IOError",
            desc: "No message has that UID, the folder `$folder` does not exist, the server does \
                   not support moving a single message, or the server does not answer.",
        },
    ],
};

/// `markSeen`'s card.
const MARK_SEEN_DOC: MethodDoc = MethodDoc {
    short: "Marks the message `$uid` as read. It sets the `\\Seen` flag.",
    params: &[UID_PARAM],
    ret: "Nothing.",
    errors: &[UID_ERROR, CHANGE_ERROR],
};

/// `markUnseen`'s card.
const MARK_UNSEEN_DOC: MethodDoc = MethodDoc {
    short: "Marks the message `$uid` as not read. It removes the `\\Seen` flag.",
    params: &[UID_PARAM],
    ret: "Nothing.",
    errors: &[UID_ERROR, CHANGE_ERROR],
};

/// `flag`'s card.
const FLAG_DOC: MethodDoc = MethodDoc {
    short: "Adds the flag `$flag` to the message `$uid`. If the message has it already, nothing \
            changes.",
    params: &[UID_PARAM, FLAG_PARAM],
    ret: "Nothing.",
    errors: &[UID_ERROR, FLAG_ERROR, CHANGE_ERROR],
};

/// `unflag`'s card.
const UNFLAG_DOC: MethodDoc = MethodDoc {
    short: "Removes the flag `$flag` from the message `$uid`. If the message does not have it, \
            nothing changes.",
    params: &[UID_PARAM, FLAG_PARAM],
    ret: "Nothing.",
    errors: &[UID_ERROR, FLAG_ERROR, CHANGE_ERROR],
};

/// `delete`'s card.
const DELETE_DOC: MethodDoc = MethodDoc {
    short: "Deletes the message `$uid` from the folder at once. No other message is deleted.",
    params: &[UID_PARAM],
    ret: "Nothing.",
    errors: &[
        UID_ERROR,
        ErrorDoc {
            error: "IOError",
            desc: "No message has that UID, the server does not support deleting a single \
                   message, or the server does not answer.",
        },
    ],
};

/// What one open mailbox talks to: a server, or the directory of a `directory` block.
#[expect(
    clippy::large_enum_variant,
    reason = "one per open mailbox, and `Held` is boxed already when it is filed"
)]
enum Backend {
    /// A signed-in session.
    Server(Session),
    /// A `directory` block's folders on disk.
    Directory(Directory),
}

/// One open session, as a request holds it.
pub(crate) struct Held {
    /// The server or the directory.
    mailbox: Backend,
    /// The folder selected last, and its `UIDVALIDITY`.
    selected: Option<(String, u32)>,
    /// How long one call may wait: the block's `timeout`.
    timeout: Duration,
}

impl std::fmt::Debug for Held {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Held")
            .field("selected", &self.selected)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

impl nvs_runtime::HeldConnection for Held {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

impl Drop for Held {
    /// Sends `LOGOUT` to a server. The deadline is now, so a server that does not read it costs
    /// no wait. A directory has nothing to close.
    fn drop(&mut self) {
        if let Backend::Server(session) = &mut self.mailbox {
            session.set_deadline(Instant::now());
            session.logout();
        }
    }
}

impl Held {
    /// Bounds the next call to a server by the block's `timeout`.
    fn start(&mut self) {
        if let Backend::Server(session) = &mut self.mailbox {
            session.set_deadline(Instant::now() + self.timeout);
        }
    }

    /// The directory of a `directory` block, which a change needs `fs.write` for.
    fn directory_root(&self) -> Option<PathBuf> {
        match &self.mailbox {
            Backend::Directory(directory) => Some(directory.root().to_path_buf()),
            Backend::Server(_) => None,
        }
    }

    /// The signed-in session of a server block.
    fn server(&mut self, member: &str) -> Result<&mut Session, Fault> {
        match &mut self.mailbox {
            Backend::Server(session) => Ok(session),
            Backend::Directory(_) => Err(Fault::fatal(format!(
                "{member}: a directory mailbox was asked for its server session"
            ))),
        }
    }

    /// The name of the selected folder.
    fn current(&self, member: &str) -> Result<String, Fault> {
        self.selected
            .as_ref()
            .map(|(name, _)| name.clone())
            .ok_or_else(|| Fault::fatal(format!("{member}: no folder is selected")))
    }
}

// Each call below takes the session's key and not the `Held` itself. A directory block's files are
// reached through the doors of `nvs_runtime::capability`, and a door reads `ctx`, so the
// [`Directory`] is copied out of its session first. A server's session is borrowed as it is.

/// A copy of session `key`'s [`Directory`], or `None` for a server block.
fn directory_of(ctx: &mut Ctx, key: u64, member: &str) -> Result<Option<Directory>, Fault> {
    Ok(match &held(ctx, key, member)?.mailbox {
        Backend::Directory(directory) => Some(directory.clone()),
        Backend::Server(_) => None,
    })
}

/// Every folder of session `key`'s mailbox.
fn folders(ctx: &mut Ctx, key: u64, member: &str) -> Result<Vec<String>, Fault> {
    if let Some(directory) = directory_of(ctx, key, member)? {
        return directory.folders(ctx, member);
    }
    held(ctx, key, member)?.server(member)?.folders(member)
}

/// Selects `folder` unless it is selected already, and returns its `UIDVALIDITY`.
fn select(ctx: &mut Ctx, key: u64, folder: &str, member: &str) -> Result<u32, Fault> {
    let session = held(ctx, key, member)?;
    if let Some((name, validity)) = &session.selected
        && name == folder
    {
        return Ok(*validity);
    }
    session.selected = None;
    let validity = match directory_of(ctx, key, member)? {
        Some(directory) => directory.select(ctx, folder, member)?,
        None => held(ctx, key, member)?
            .server(member)?
            .select(folder, member)?,
    };
    held(ctx, key, member)?.selected = Some((folder.to_owned(), validity));
    Ok(validity)
}

/// The UIDs of the selected folder that match `filter`. `ceiling` is how many bytes the request
/// can still hold.
fn search(
    ctx: &mut Ctx,
    key: u64,
    filter: &Filter,
    ceiling: u64,
    member: &str,
) -> Result<Vec<u32>, Fault> {
    let folder = held(ctx, key, member)?.current(member)?;
    if let Some(directory) = directory_of(ctx, key, member)? {
        return directory.search(ctx, &folder, filter, ceiling, member);
    }
    held(ctx, key, member)?
        .server(member)?
        .search(filter, member)
}

/// Adds `flag` to message `uid` of the selected folder, or removes it when `add` is false.
fn store(
    ctx: &mut Ctx,
    key: u64,
    uid: u32,
    flag: &str,
    add: bool,
    member: &str,
) -> Result<(), Fault> {
    let folder = held(ctx, key, member)?.current(member)?;
    if let Some(directory) = directory_of(ctx, key, member)? {
        return directory.store(ctx, &folder, uid, flag, add, member);
    }
    held(ctx, key, member)?
        .server(member)?
        .store(uid, flag, add, member)
}

/// Moves message `uid` of the selected folder to `target`.
fn move_to(ctx: &mut Ctx, key: u64, uid: u32, target: &str, member: &str) -> Result<(), Fault> {
    let folder = held(ctx, key, member)?.current(member)?;
    if let Some(directory) = directory_of(ctx, key, member)? {
        return directory.move_to(ctx, &folder, uid, target, member);
    }
    held(ctx, key, member)?
        .server(member)?
        .move_to(uid, target, member)
}

/// Deletes message `uid` of the selected folder.
fn delete(ctx: &mut Ctx, key: u64, uid: u32, member: &str) -> Result<(), Fault> {
    let folder = held(ctx, key, member)?.current(member)?;
    if let Some(directory) = directory_of(ctx, key, member)? {
        return directory.delete(ctx, &folder, uid, member);
    }
    held(ctx, key, member)?.server(member)?.delete(uid, member)
}

/// Selects `folder` and checks that its `UIDVALIDITY` is still `expected`.
fn reselect(
    ctx: &mut Ctx,
    key: u64,
    folder: &str,
    expected: u32,
    member: &str,
) -> Result<(), Fault> {
    let validity = select(ctx, key, folder, member)?;
    if validity == expected {
        return Ok(());
    }
    Err(io(format!(
        "{member}: the `UIDVALIDITY` of the folder `{folder}` changed from {expected} to \
         {validity}, so its UIDs name other messages now. Open the folder again."
    )))
}

/// Message `uid` of the selected folder, parsed, with `uid()` set. `ceiling` is how many bytes
/// the request can still hold.
fn fetch(ctx: &mut Ctx, key: u64, uid: u32, ceiling: u64, member: &str) -> Result<Value, Fault> {
    let folder = held(ctx, key, member)?.current(member)?;
    let raw = match directory_of(ctx, key, member)? {
        Some(directory) => directory.fetch(ctx, &folder, uid, ceiling, member)?,
        None => held(ctx, key, member)?
            .server(member)?
            .fetch(uid, ceiling, member)?,
    };
    Ok(crate::mail::receive::fetched_message(&raw, uid))
}

/// An `IOError` with `message`.
fn io(message: String) -> Fault {
    Fault::thrown_as(ThrownClass::Io, message)
}

/// What `[imap.<name>]` said, once `open` has checked it.
struct Block {
    server: Server,
    port: u16,
    timeout: Duration,
    /// The password sign-in, or `None` for an `xoauth2` block.
    password: Option<(String, String)>,
    /// The grant of an `xoauth2` block.
    oauth: Option<oauth::Grant>,
}

/// What `[imap.<name>]` names: a server, or the directory of a `directory` block.
#[expect(
    clippy::large_enum_variant,
    reason = "made once per `open` and taken apart at once"
)]
enum Opened {
    Server(Block),
    Directory(PathBuf),
}

/// The block `endpoint` names, once `imap.connect` has been shown to grant it. The grant is
/// asked first, so a program with no grant learns nothing about which blocks exist. A
/// `directory` block needs `fs.read` for its directory too.
///
/// # Errors
///
/// A catchable `RuntimeError` for an ungranted name or directory, a block with no `host`, a
/// block with no sign-in or half of one, and a word the boot would have refused.
fn block_of(ctx: &Ctx, endpoint: &str, member: &str) -> Result<Opened, Fault> {
    nvs_runtime::capability::require(
        ctx,
        nvs_config::Cap::ImapConnect,
        Scope::Name(endpoint),
        member,
    )?;
    let read = |key: &str| configured_in(ctx, "imap", endpoint, key);
    if let Some(directory) = read("directory") {
        let root = PathBuf::from(directory);
        nvs_runtime::capability::require(ctx, nvs_config::Cap::FsRead, Scope::Path(&root), member)?;
        let found = nvs_runtime::capability::metadata_if_present(ctx, &root, member)?;
        if !found.is_some_and(|found| found.is_dir()) {
            return Err(io(format!(
                "{member}: the directory {} of `[imap.{endpoint}]` does not exist",
                root.display()
            )));
        }
        return Ok(Opened::Directory(root));
    }
    let Some(host) = read("host") else {
        return Err(Fault::thrown(format!(
            "{member}: no `[imap.{endpoint}]` block sets `host`, so there is no mailbox of that \
             name to open"
        )));
    };
    let Ok(auth) = nvs_config::mail::auth_from(read("auth").as_deref()) else {
        return Err(Fault::thrown(format!(
            "{member}: `[imap.{endpoint}] auth` is not `plain`, `login` or `xoauth2`"
        )));
    };
    let Some(tls) = nvs_config::imap::tls_from(read("tls").as_deref()) else {
        return Err(Fault::thrown(format!(
            "{member}: `[imap.{endpoint}] tls` is not `implicit` or `starttls`"
        )));
    };
    let oauth = if auth == Some(Auth::XOAuth2) {
        Some(oauth::grant_of(ctx, "imap", endpoint, member)?)
    } else {
        None
    };
    let password = match (read("user"), read("password")) {
        _ if oauth.is_some() => None,
        (Some(user), Some(password)) => Some((user, password)),
        (None, None) => {
            return Err(Fault::thrown(format!(
                "{member}: `[imap.{endpoint}]` sets no `user` and `password`, and a mailbox \
                 always needs a sign-in"
            )));
        }
        (user, _) => {
            let (written, missing) = if user.is_some() {
                ("user", "password")
            } else {
                ("password", "user")
            };
            return Err(Fault::thrown(format!(
                "{member}: `[imap.{endpoint}]` sets `{written}` and no `{missing}`, and half a \
                 sign-in is not one"
            )));
        }
    };
    let port = read("port")
        .and_then(|text| text.parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(nvs_config::defaults::IMAP_PORT);
    let timeout = read("timeout")
        .and_then(|text| duration::parse(&text).ok())
        .map(|nanos| Duration::from_nanos(nanos.unsigned_abs()))
        .filter(|bound| !bound.is_zero())
        .unwrap_or(nvs_config::defaults::IMAP_TIMEOUT);
    Ok(Opened::Server(Block {
        server: Server {
            name: endpoint.to_owned(),
            host,
            tls,
            ca_file: read("tls_ca_file").map(PathBuf::from),
            auth,
        },
        port,
        timeout,
        password,
        oauth,
    }))
}

/// `Core\Imap::open`'s body: the key the new session is held under.
fn open(ctx: &mut Ctx, endpoint: &str) -> Result<u64, Fault> {
    let block = match block_of(ctx, endpoint, OPEN)? {
        Opened::Server(block) => block,
        Opened::Directory(root) => {
            let held = Held {
                mailbox: Backend::Directory(Directory::new(root, endpoint.to_owned())),
                selected: None,
                timeout: nvs_config::defaults::IMAP_TIMEOUT,
            };
            return Ok(ctx.hold_open_connection(None, None, Box::new(held)));
        }
    };
    let now = Instant::now();
    let until = now + block.timeout;
    let lookup_until = ctx
        .off_core_deadline()
        .map_or(until, |wall| until.min(wall));
    // The operator wrote this host, so it is resolved with no address policy in front of it,
    // as a `[mail.<name>]` host is.
    let address =
        nvs_runtime::capability::resolve_host(&block.server.host, OPEN, Some(lookup_until))
            .map(|address| SocketAddr::new(address, block.port))
            .map_err(|_| {
                io(format!(
                    "{OPEN} could not resolve {}, which `[imap.{endpoint}]` names",
                    block.server.host
                ))
            })?;
    let label = format!("imap.{endpoint}");
    let sign_in = match (&block.password, &block.oauth) {
        (_, Some(grant)) => SignIn::Token {
            user: grant.user.clone(),
            token: oauth::token(grant, &label, until, OPEN)?,
        },
        (Some((user, password)), None) => SignIn::Password {
            user: user.clone(),
            password: password.clone(),
        },
        (None, None) => {
            return Err(Fault::fatal(format!(
                "{OPEN}: `[imap.{endpoint}]` was read with no sign-in"
            )));
        }
    };
    let session =
        Session::open(address, &block.server, &sign_in, until, OPEN).inspect_err(|_| {
            // A token the server refused is fetched again by the next `open`.
            if block.oauth.is_some() {
                oauth::forget(&label);
            }
        })?;
    let held = Held {
        mailbox: Backend::Server(session),
        selected: None,
        timeout: block.timeout,
    };
    Ok(ctx.hold_open_connection(None, None, Box::new(held)))
}

/// The session the request holds under `key`.
///
/// # Errors
///
/// A `LogicError` for a key that names no open session.
fn held<'c>(ctx: &'c mut Ctx, key: u64, member: &str) -> Result<&'c mut Held, Fault> {
    ctx.open_connection_mut(key)
        .and_then(|conn| conn.as_any_mut().downcast_mut::<Held>())
        .ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!("{member}: this mailbox is closed"),
            )
        })
}

/// The key, the block name and, for a folder, its name and `UIDVALIDITY`, of `receiver`.
struct Receiver {
    key: u64,
    block: String,
    folder: Option<(String, u32)>,
}

/// What `value`, a [`MAILBOX`] or a [`FOLDER_CLASS`], holds.
fn receiver_of(value: Value, class: &'static CoreClass, member: &str) -> Result<Receiver, Fault> {
    let object = crate::instance::receiver(value, class, member)?;
    let broken = || {
        Fault::fatal(format!(
            "{member} found a `{}` it did not build",
            class.name
        ))
    };
    let key = crate::instance::slot(object, HANDLE)
        .as_uint()
        .ok_or_else(broken)?;
    let block = crate::instance::slot(object, BLOCK)
        .as_text()
        .ok_or_else(broken)?
        .to_owned();
    let folder = if class.slots.len() > FOLDER {
        let name = crate::instance::slot(object, FOLDER)
            .as_text()
            .ok_or_else(broken)?
            .to_owned();
        let validity = crate::instance::slot(object, VALIDITY)
            .as_int()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(broken)?;
        Some((name, validity))
    } else {
        None
    };
    Ok(Receiver { key, block, folder })
}

/// The key of the session a [`FOLDER_CLASS`] receiver names, with its folder selected.
fn folder_session(ctx: &mut Ctx, value: Value, member: &str) -> Result<u64, Fault> {
    let Receiver { key, folder, .. } = receiver_of(value, &FOLDER_CLASS, member)?;
    let (name, validity) = folder.ok_or_else(|| Fault::fatal(format!("{member}: no folder")))?;
    held(ctx, key, member)?.start();
    reselect(ctx, key, &name, validity, member)?;
    Ok(key)
}

/// [`folder_session`], once `fs.write` grants the directory of a `directory` block.
fn writing_folder(ctx: &mut Ctx, value: Value, member: &str) -> Result<u64, Fault> {
    let key = receiver_of(value, &FOLDER_CLASS, member)?.key;
    if let Some(root) = held(ctx, key, member)?.directory_root() {
        nvs_runtime::capability::require(
            ctx,
            nvs_config::Cap::FsWrite,
            Scope::Path(&root),
            member,
        )?;
    }
    folder_session(ctx, value, member)
}

/// `$flag` as the flag a change sends: `\Answered`, `\Flagged`, `\Draft` and `\Seen` in that
/// case, and a keyword as written.
///
/// # Errors
///
/// A catchable `RuntimeError` for a flag over [`MAX_KEYWORD_BYTES`], for `\Deleted`, which only
/// `delete` sets, for any other name that starts with `\`, and for text that is not an IMAP atom.
pub(super) fn flag_of(text: &str, member: &str) -> Result<String, Fault> {
    use imap_codec::imap_types::flag::Flag;
    let refused =
        |why: &str| Fault::thrown(format!("{member}: `{text}` cannot be set as a flag. {why}"));
    if text.len() > MAX_KEYWORD_BYTES {
        return Err(Fault::thrown(format!(
            "{member}: a flag is at most {MAX_KEYWORD_BYTES} bytes, and this one is {} bytes.",
            text.len()
        )));
    }
    match Flag::try_from(text) {
        Ok(Flag::Deleted) => Err(refused("To delete a message, call `delete`.")),
        Ok(Flag::Keyword(_)) => Ok(text.to_owned()),
        Ok(Flag::Extension(_)) | Err(_) if text.starts_with('\\') => Err(refused(
            "A flag that starts with `\\` is `\\Answered`, `\\Flagged`, `\\Draft` or `\\Seen`.",
        )),
        Ok(flag) => Ok(flag.to_string()),
        Err(_) => Err(refused(
            "A keyword has no space and none of the characters `(){%*\"\\]`.",
        )),
    }
}

/// The text at `args[at]`, or `None` for an option the call left out.
fn optional_text(
    args: &[Value],
    at: usize,
    what: &str,
    member: &str,
) -> Result<Option<String>, Fault> {
    if matches!(args[at].tag(), Some(Tag::Null | Tag::Unset)) {
        return Ok(None);
    }
    args[at]
        .as_text()
        .map(|text| Some(text.to_owned()))
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member} expected a `string` for `{what}`, got tag {}",
                args[at].tag_byte()
            ))
        })
}

/// The day of the `Core\Time\Date` at `args[at]`, or `None` for an option the call left out.
fn optional_day(args: &[Value], at: usize, member: &str) -> Result<Option<(i32, u32, u32)>, Fault> {
    if matches!(args[at].tag(), Some(Tag::Null | Tag::Unset)) {
        return Ok(None);
    }
    let date = crate::time::date_of(args, at, member)?;
    let month = u32::try_from(date.month()).unwrap_or(1);
    let day = u32::try_from(date.day()).unwrap_or(1);
    Ok(Some((i32::from(date.year()), month, day)))
}

/// `search`'s options as a [`Filter`].
fn filter_of(args: &[Value], member: &str) -> Result<Filter, Fault> {
    let unseen = args[UNSEEN].as_bool().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a `bool` for `unseen`, got tag {}",
            args[UNSEEN].tag_byte()
        ))
    })?;
    let limit = if matches!(args[LIMIT].tag(), Some(Tag::Null | Tag::Unset)) {
        None
    } else {
        let written = args[LIMIT].as_int().ok_or_else(|| {
            Fault::fatal(format!(
                "{member} expected an `int` for `limit`, got tag {}",
                args[LIMIT].tag_byte()
            ))
        })?;
        if written < 1 {
            return Err(Fault::thrown(format!(
                "{member}: `limit` is {written}, and it must be 1 or more"
            )));
        }
        Some(usize::try_from(written).unwrap_or(usize::MAX))
    };
    Ok(Filter {
        unseen,
        since: optional_day(args, SINCE, member)?,
        before: optional_day(args, BEFORE, member)?,
        from: optional_text(args, FROM, "from", member)?,
        subject: optional_text(args, SUBJECT, "subject", member)?,
        limit,
    })
}

/// `$uid` as a UID, which is 1 to `u32::MAX`.
fn uid_of(value: Value, member: &str) -> Result<u32, Fault> {
    let written = value.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected an `int` for `$uid`, got tag {}",
            value.tag_byte()
        ))
    })?;
    u32::try_from(written)
        .ok()
        .filter(|uid| *uid != 0)
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{member}: `$uid` is {written}, and a UID is 1 to 4294967295"
            ))
        })
}

nvs_runtime::nvs_helper! {
    /// `Core\Imap::open(string $endpoint): Core\Imap\Mailbox`.
    fn nvs_core_imap_open(ctx, args: [1]) {
        let endpoint = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!("{OPEN} expected a `string` for `$endpoint`, got tag {}", args[0].tag_byte()))
        })?.to_owned();
        let key = open(ctx, &endpoint)?;
        Ok(crate::instance::build(&MAILBOX, [Value::uint(key), Value::text(&endpoint)]))
    }
}

nvs_runtime::nvs_helper! {
    /// `$mailbox->folders(): array<string>`.
    fn nvs_core_imap_mailbox_folders(ctx, args: [1]) {
        let member = format!("{MAILBOX_NAME}::folders");
        let Receiver { key, .. } = receiver_of(args[0], &MAILBOX, &member)?;
        held(ctx, key, &member)?.start();
        let mut out = NvsArray::new();
        for name in folders(ctx, key, &member)? {
            out.append(Value::text(&name));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `$mailbox->folder(string $name): Core\Imap\Folder` — selects the folder.
    fn nvs_core_imap_mailbox_folder(ctx, args: [2]) {
        let member = format!("{MAILBOX_NAME}::folder");
        let Receiver { key, block, .. } = receiver_of(args[0], &MAILBOX, &member)?;
        let name = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!("{member} expected a `string` for `$name`, got tag {}", args[1].tag_byte()))
        })?.to_owned();
        held(ctx, key, &member)?.start();
        let validity = select(ctx, key, &name, &member)?;
        Ok(crate::instance::build(
            &FOLDER_CLASS,
            [
                Value::uint(key),
                Value::text(&block),
                Value::text(&name),
                Value::int(i64::from(validity)),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$folder->uidValidity(): int` — the value `folder` read.
    fn nvs_core_imap_folder_uid_validity(_ctx, args: [1]) {
        crate::instance::read_slot(args, &FOLDER_CLASS, VALIDITY, "uidValidity")
    }
}

nvs_runtime::nvs_helper! {
    /// `$folder->search({unseen?, since?, before?, from?, subject?, limit?}): array<int>`.
    fn nvs_core_imap_folder_search(ctx, args: [7]) {
        let member = format!("{FOLDER_NAME}::search");
        let filter = filter_of(args, &member)?;
        let key = folder_session(ctx, args[0], &member)?;
        let mut out = NvsArray::new();
        for uid in search(ctx, key, &filter, headroom(), &member)? {
            out.append(Value::int(i64::from(uid)));
        }
        Ok(Value::array(out))
    }
}

/// The memory the request has left, in bytes: the most one read message may take.
fn headroom() -> u64 {
    nvs_runtime::budget::headroom().map_or(u64::MAX, |room| u64::try_from(room).unwrap_or(u64::MAX))
}

/// The body of `markSeen`, `markUnseen`, `flag` and `unflag`: `flag` added to message `$uid`,
/// or removed when `add` is false.
fn change_flag(
    ctx: &mut Ctx,
    args: &[Value],
    flag: &str,
    add: bool,
    member: &str,
) -> Result<Value, Fault> {
    let uid = uid_of(args[1], member)?;
    let flag = flag_of(flag, member)?;
    let key = writing_folder(ctx, args[0], member)?;
    store(ctx, key, uid, &flag, add, member)?;
    Ok(Value::null())
}

/// The text of `$flag`, the second argument of `flag` and `unflag`.
fn flag_text(args: &[Value], member: &str) -> Result<String, Fault> {
    args[2].as_text().map(str::to_owned).ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a `string` for `$flag`, got tag {}",
            args[2].tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `$folder->move(int $uid, string $folder): void`.
    fn nvs_core_imap_folder_move(ctx, args: [3]) {
        let member = format!("{FOLDER_NAME}::move");
        let uid = uid_of(args[1], &member)?;
        let target = args[2].as_text().ok_or_else(|| {
            Fault::fatal(format!("{member} expected a `string` for `$folder`, got tag {}", args[2].tag_byte()))
        })?.to_owned();
        let key = writing_folder(ctx, args[0], &member)?;
        move_to(ctx, key, uid, &target, &member)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$folder->markSeen(int $uid): void` — sets `\Seen`.
    fn nvs_core_imap_folder_mark_seen(ctx, args: [2]) {
        change_flag(ctx, args, "\\Seen", true, &format!("{FOLDER_NAME}::markSeen"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$folder->markUnseen(int $uid): void` — removes `\Seen`.
    fn nvs_core_imap_folder_mark_unseen(ctx, args: [2]) {
        change_flag(ctx, args, "\\Seen", false, &format!("{FOLDER_NAME}::markUnseen"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$folder->flag(int $uid, string $flag): void`.
    fn nvs_core_imap_folder_flag(ctx, args: [3]) {
        let member = format!("{FOLDER_NAME}::flag");
        let flag = flag_text(args, &member)?;
        change_flag(ctx, args, &flag, true, &member)
    }
}

nvs_runtime::nvs_helper! {
    /// `$folder->unflag(int $uid, string $flag): void`.
    fn nvs_core_imap_folder_unflag(ctx, args: [3]) {
        let member = format!("{FOLDER_NAME}::unflag");
        let flag = flag_text(args, &member)?;
        change_flag(ctx, args, &flag, false, &member)
    }
}

nvs_runtime::nvs_helper! {
    /// `$folder->delete(int $uid): void` — `\Deleted` and `UID EXPUNGE` of that one UID.
    fn nvs_core_imap_folder_delete(ctx, args: [2]) {
        let member = format!("{FOLDER_NAME}::delete");
        let uid = uid_of(args[1], &member)?;
        let key = writing_folder(ctx, args[0], &member)?;
        delete(ctx, key, uid, &member)?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$folder->fetch(int $uid): Core\Mail\Message`.
    fn nvs_core_imap_folder_fetch(ctx, args: [2]) {
        let member = format!("{FOLDER_NAME}::fetch");
        let uid = uid_of(args[1], &member)?;
        let key = folder_session(ctx, args[0], &member)?;
        // The raw message and its parsed copy are both held, so the ceiling is the memory the
        // request has left.
        fetch(ctx, key, uid, headroom(), &member)
    }
}

/// This module's arm of the `Core` symbol table — see [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    let helper: nvs_runtime::NvsFn = match symbol {
        "nvs_core_imap_open" => nvs_core_imap_open,
        "nvs_core_imap_mailbox_folders" => nvs_core_imap_mailbox_folders,
        "nvs_core_imap_mailbox_folder" => nvs_core_imap_mailbox_folder,
        "nvs_core_imap_folder_uid_validity" => nvs_core_imap_folder_uid_validity,
        "nvs_core_imap_folder_search" => nvs_core_imap_folder_search,
        "nvs_core_imap_folder_fetch" => nvs_core_imap_folder_fetch,
        "nvs_core_imap_folder_move" => nvs_core_imap_folder_move,
        "nvs_core_imap_folder_mark_seen" => nvs_core_imap_folder_mark_seen,
        "nvs_core_imap_folder_mark_unseen" => nvs_core_imap_folder_mark_unseen,
        "nvs_core_imap_folder_flag" => nvs_core_imap_folder_flag,
        "nvs_core_imap_folder_unflag" => nvs_core_imap_folder_unflag,
        "nvs_core_imap_folder_delete" => nvs_core_imap_folder_delete,
        _ => return None,
    };
    Some(nvs_runtime::Entry::borrowed((helper as *const ()).cast()))
}
