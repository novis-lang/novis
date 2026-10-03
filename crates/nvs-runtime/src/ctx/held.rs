//! What a request holds open, and gives back when it ends.
//!
//! Started scripts, open files, open sockets and open connections: a table of
//! handles each, where a member takes one out and hands back a key, so that a
//! value in Novis code is a number rather than a pointer and a handle awaited
//! twice reads an empty slot rather than another request's resource.
//!
//! And one thing that is not a handle at all — the temporary directories
//! `rule:core-classes/temporary-dir-sweep` has the runtime delete when the script ends. It is here for the second
//! half of this file's title rather than the first: nothing in Novis holds a key
//! to one, but the request gives them back when it ends exactly as it gives back
//! every table above. [`Ctx::track_temporary_dir`] is the one writer.
//!
//! [`HeldConnection`], [`HeldSocket`] and [`HeldReader`] are the traits that let
//! this crate hold something it may not name: a
//! `rule:core-classes/db-drivers-are-an-enum` driver's connection,
//! `nvs-host`'s parking socket, and the half-read reply body `nvs-stdlib`
//! frames a streamed call off. Every one of those dependencies runs the other
//! way, so each field is a `dyn Trait` and the only method on any of them is the
//! downcast a holder needs to get its own type back.

use super::*;

/// A database connection a request is holding open — `nvs_db`'s `Connection`,
/// and the seam that lets a [`Ctx`] hold one without naming it.
///
/// Declared here for [`Running`](crate::host::Running)'s reason and one more.
/// A `Core\Db\Connection` is an object with no native drop, so the connection
/// itself is a key into a table the request owns
/// ([`Ctx::hold_open_connection`]); the table has to live in this crate,
/// because this is the crate that learns when a request ends. And the edge
/// cannot run the other way: `rule:core-classes/db-crate-boundary` has `nvs-db` depending on this crate, so a field typed
/// `nvs_db::Connection` would close a cycle.
///
/// The one method is the downcast a holder needs to get its own type back,
/// which `dyn Trait` cannot do on its own. Nothing in this crate calls it —
/// what this crate wants from a connection is that it is dropped with the
/// request, which is [`Drop`]'s job and needs no method at all.
pub trait HeldConnection: std::fmt::Debug + std::any::Any {
    /// This connection as the concrete type its driver crate knows it by.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// The same downcast, owning — what a caller taking a connection out of
    /// [`crate::pool`] needs, because
    /// `rule:security/db-pool-reset-is-a-boundary`'s reset consumes the
    /// connection so that a failed one cannot be handed back.
    ///
    /// No default body: it would have to coerce `Self` to `dyn Any`, which a
    /// trait's own body cannot do without knowing `Self: Sized`. Every impl is
    /// `self`, one line.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;

    /// Whether this connection may rejoin the core's pool at teardown —
    /// `rule:security/db-pool-reset-is-a-boundary`'s release gate, asked
    /// of the driver because only the driver knows where its wire is.
    ///
    /// **The default is `false`**, which is § 13's "a driver with no reset
    /// primitive is not poolable at all" written as the answer a driver gets
    /// for saying nothing. A driver that has not decided is one whose
    /// connections are closed with the request.
    ///
    /// This is a state question and never an I/O one: it is asked from inside
    /// [`Drop`], where nothing may wait. The reset itself is the acquiring
    /// request's, and [`crate::pool`]'s module doc owns why.
    fn is_poolable(&self) -> bool {
        false
    }
}

/// A socket a request is holding open — `nvs-host`'s parking stream or its
/// listening half, and the seam that lets a [`Ctx`] hold one without naming it.
///
/// Declared here for [`HeldConnection`]'s reason, which is the same edge run
/// the same way: `nvs-host` depends on this crate, so a field typed
/// `nvs_host::NvsTcp` would close a cycle. And the table has to live here
/// because this is the crate that learns when a request ends, which is when
/// `rule:core-classes/net-one-api-three-transports`'s socket lifetime says
/// every socket the request opened is closed.
///
/// The one method is the downcast a holder needs to get its own type back,
/// which `dyn Trait` cannot do on its own. Nothing in this crate calls it —
/// what this crate wants from a socket is that it is dropped with the request,
/// which is [`Drop`]'s job and needs no method at all.
pub trait HeldSocket: std::fmt::Debug + std::any::Any {
    /// This socket as the concrete type the crate that opened it knows it by.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// A reply body a request is still reading — the reader `nvs-stdlib` frames a
/// streamed outbound call off, and the seam that lets a [`Ctx`] hold one
/// without naming it.
///
/// [`HeldSocket`]'s edge, run for the same reason: `nvs-stdlib` depends on this
/// crate, so a field typed after its reader would close a cycle. The table has
/// to live here because this is the crate that learns when a request ends,
/// which is when the connection the rest of a half-read reply would have
/// arrived on is closed.
///
/// The one method is the downcast a holder needs to get its own type back,
/// which `dyn Trait` cannot do on its own. Nothing in this crate calls it —
/// what this crate wants from a reader is that it is dropped with the request.
pub trait HeldReader: std::fmt::Debug + std::any::Any {
    /// This reader as the concrete type the crate that opened it knows it by.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// One child process a task has started, with the three pipes
/// `Core\Process\Handle`'s members read and write through.
///
/// **A child never outlives the task that started it** —
/// `rule:core-classes/process-spawn`, and this type's [`Drop`] is the whole of
/// the mechanism: the child is killed and then reaped, so a handle the program
/// stopped reading from leaves neither a running process nor a zombie entry in
/// the operating system's table behind it. A bare [`std::process::Child`] drops
/// without doing either, which is why the table holds this and not the child.
///
/// A pipe is taken out for the length of one read or write and put back
/// afterwards, because the read happens on the blocking pool and
/// `nvs_host::blocking::run`'s closure owns what it reads from. Holding the
/// pipes beside the child rather than inside it is what makes that one move
/// instead of a borrow across a suspension point.
///
/// **What it spends:** one handle and up to three pipes per live child,
/// charged to the task that spawned it and released with it — O(children in
/// flight), never O(children ever started).
#[derive(Debug)]
pub struct HeldChild {
    /// The child, or `None` once [`HeldChild::reap`] has waited for it.
    child: Option<std::process::Child>,
    /// The status the child exited with, once something has waited for it.
    /// A second `wait` answers this rather than reaping twice.
    exited: Option<std::process::ExitStatus>,
    /// The child's standard input, `None` while a write holds it and once the
    /// program has closed it.
    pub stdin: Option<std::process::ChildStdin>,
    /// The child's standard output, `None` while a read holds it and once the
    /// stream has ended.
    pub stdout: Option<std::process::ChildStdout>,
    /// The child's standard error, on [`HeldChild::stdout`]'s terms.
    pub stderr: Option<std::process::ChildStderr>,
    /// `rule:observability/spawn-is-its-own-event`'s event the spawn opened,
    /// travelling with the child because the `wait` that joins it is the one
    /// place that still knows which child it was. `None` for a child started
    /// while neither trace bit was on, and `None` again once a wait has taken
    /// it — so a second wait closes nothing twice.
    pub spawn_event: Option<crate::OpenSpawn>,
    /// The instant `rule:core-classes/process-options`' `timeout` ends this
    /// child's time, or `None` for a child started without one. Every handle
    /// member reads it before it touches the child, and one that parks reads
    /// it again for as long as it is parked.
    pub deadline: Option<std::time::Instant>,
}

impl HeldChild {
    /// Takes the three pipes out of a freshly started child and holds both
    /// halves together.
    #[must_use]
    pub fn new(mut child: std::process::Child) -> Self {
        Self {
            stdin: child.stdin.take(),
            stdout: child.stdout.take(),
            stderr: child.stderr.take(),
            exited: None,
            spawn_event: None,
            deadline: None,
            child: Some(child),
        }
    }

    /// The status this child has already been seen to exit with, or `None`
    /// while it is still one nothing has waited for.
    #[must_use]
    pub fn status(&self) -> Option<std::process::ExitStatus> {
        self.exited
    }

    /// Asks the operating system to end the child, and answers whether there
    /// was still one to end.
    ///
    /// Idempotent on purpose: `Core\Process\Handle::kill` is `void`, and a
    /// program that kills a child which has already exited has got what it
    /// asked for. The status is not collected here — [`HeldChild::reap`] is
    /// what a `wait` calls, and killing without waiting leaves exactly the
    /// zombie this type's [`Drop`] exists to prevent.
    ///
    /// # Errors
    ///
    /// Whatever the operating system said about the signal.
    pub fn kill(&mut self) -> std::io::Result<bool> {
        let Some(child) = self.child.as_mut() else {
            return Ok(false);
        };
        child.kill()?;
        Ok(true)
    }

    /// Waits for the child to exit and answers its status, **blocking the
    /// calling thread** — so this is only ever called inside
    /// `nvs_host::blocking::run`'s closure, which is why this type is moved
    /// out of the table for a `wait` rather than borrowed from it.
    ///
    /// The standard input pipe is dropped first. A child reading to the end of
    /// its input never sees one while this process still holds the writing end,
    /// so waiting without closing it is the deadlock every `proc_open` port
    /// eventually writes.
    ///
    /// # Errors
    ///
    /// Whatever the operating system said about reaping the child.
    pub fn reap(&mut self) -> std::io::Result<std::process::ExitStatus> {
        if let Some(status) = self.exited {
            return Ok(status);
        }
        drop(self.stdin.take());
        let status = match self.child.as_mut() {
            Some(child) => child.wait()?,
            // Unreachable while `exited` is the only thing that empties
            // `child`, and cheaper to answer than to prove at every caller.
            None => return Err(std::io::Error::other("the child was never started")),
        };
        self.child = None;
        self.exited = Some(status);
        Ok(status)
    }

    /// [`HeldChild::reap`] without blocking: the status if the child has
    /// exited, and `None` while it still runs. A wait under a timeout asks
    /// this in a loop, so it can stop the child when the timeout passes.
    ///
    /// # Errors
    ///
    /// Whatever the operating system said about the child.
    pub fn try_reap(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        if let Some(status) = self.exited {
            return Ok(Some(status));
        }
        let Some(child) = self.child.as_mut() else {
            return Err(std::io::Error::other("the child was never started"));
        };
        let Some(status) = child.try_wait()? else {
            return Ok(None);
        };
        self.child = None;
        self.exited = Some(status);
        Ok(Some(status))
    }
}

impl Drop for HeldChild {
    fn drop(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        // Both, and in this order. The kill is what makes the child not outlive
        // the task; the wait is what keeps the operating system's process table
        // from filling with entries nobody will ever collect. Neither answer is
        // actionable here — the task this child belonged to is already ending.
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// One connection a request has open: how it is reached again within the
/// request, where it goes when the request ends, and the connection itself.
///
/// The lease is `None` for a connection that is closed with the request and
/// never pooled — an embedder's, in which case there is nothing for
/// [`crate::pool`] to be handed and nothing it is counted against.
#[derive(Debug)]
pub(super) struct OpenConnection {
    /// § 2's memoization key, or `None` for a `{shared: false}` call — see
    /// [`Ctx::hold_open_connection`]. Distinct from the lease's key on
    /// purpose: `{shared: false}` bypasses memoization *within* the request and
    /// is still drawn from and returned to the pool.
    memo: Option<String>,
    /// The slot this connection is live under, holding the key and the bounds
    /// the pool needs to take it back at teardown.
    ///
    /// Taken by [`Ctx::close_open_connection`], which is what makes the
    /// release happen there instead of at teardown.
    pub(super) lease: Option<crate::pool::Lease>,
    /// The connection, held as the trait object for the reason
    /// [`HeldConnection`]'s own doc gives — and `None` once
    /// [`Ctx::close_open_connection`] has released it.
    ///
    /// **The entry stays behind the connection it no longer holds.** Keys are
    /// positions in this vector, so removing one would renumber every handle a
    /// program is still holding; and an emptied entry is the only thing that
    /// can tell a `Core\Db\Connection::close`d handle from a key this request
    /// never filed, which are a `LogicError` and a paste error respectively.
    pub(super) connection: Option<Box<dyn HeldConnection>>,
    /// The shallowest transaction level a job was enqueued under on this
    /// connection and is still undecided at, or `None` while no enqueue waits
    /// on a commit. [`Ctx::note_enqueued_under`] owns the reading.
    enqueued_under: Option<u32>,
}

impl Ctx {
    /// Files a started isolate against this request and answers the key that
    /// takes it back out — what a `Core\Script\Handle`'s one slot holds.
    ///
    /// A handle is an object and a `Core` instance has no native drop, so a
    /// key into a table is the only representation available and *whose* table
    /// it is is the whole question. It is the request's, so that the footprint
    /// is O(isolates this request has started and not awaited) and is released
    /// with the request — a process-wide table would be O(spawns served),
    /// which `rule:programs/memory-priority` calls
    /// a leak rather than a trade. `crates/nvs-stdlib/src/channel.rs` records
    /// the same reasoning for the queue it keeps in slots instead.
    ///
    /// **What it spends:** one pointer pair per live handle, and nothing at all
    /// for a request that spawns none. A key is never reused, so a handle
    /// awaited twice reads an empty slot rather than another request's isolate.
    pub fn hold_started_script(&mut self, running: Box<dyn crate::host::Running>) -> u64 {
        self.started_scripts.push(Some(running));
        // The index, one-based, so that a handle slot never holds a key a
        // zeroed value could be mistaken for.
        self.started_scripts.len() as u64
    }

    /// Takes the isolate `key` names back out, or `None` when it has already
    /// been taken — an `await` of a handle a previous `await` consumed.
    #[must_use]
    pub fn take_started_script(&mut self, key: u64) -> Option<Box<dyn crate::host::Running>> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.started_scripts.get_mut(index)?.take()
    }

    /// Files an open file against this request and answers the key that reads
    /// it back — what a `Core\IO\File`'s one slot holds.
    ///
    /// The same shape and the same reasoning as [`Ctx::hold_started_script`],
    /// which is the one home of *why* a `Core` handle is a key into a
    /// request-owned table rather than the native thing itself: a `Core`
    /// instance has no native drop, so a table this module does not own would
    /// never learn that the last reference had gone. What this adds is the
    /// close: a descriptor is scarce in a way an awaited isolate is not, so
    /// `Core\IO\File::close` takes the handle out and drops it, and a request
    /// that forgets closes everything it opened when its `Ctx` goes.
    ///
    /// **What it spends:** one `Option<File>` — a descriptor and a niche — per
    /// `open` this request performed, *including* the ones it has since closed,
    /// because a key is never reused. That is O(opens by one request), released
    /// with the request and charged to its memory limit, and it is the price of
    /// the safety property: a stale handle reads an empty slot and throws,
    /// where a recycled key would silently address whatever file the same slot
    /// now holds. A loop opening and closing a million paths spends a few
    /// megabytes for it, which
    /// `rule:programs/memory-priority`'s ordering
    /// spends without hesitating to keep a descriptor from being confused for
    /// another.
    pub fn hold_open_file(&mut self, file: std::fs::File) -> u64 {
        self.open_files.push(Some(file));
        // The index, one-based, so that a handle slot never holds a key a
        // zeroed value could be mistaken for.
        self.open_files.len() as u64
    }

    /// The file `key` names, borrowed for one read or write, or `None` once it
    /// has been closed or if it was never this request's.
    pub fn open_file_mut(&mut self, key: u64) -> Option<&mut std::fs::File> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_files.get_mut(index)?.as_mut()
    }

    /// Takes the file `key` names back out, or `None` when it has already been
    /// taken — a `close` of a handle a previous `close` consumed.
    #[must_use]
    pub fn take_open_file(&mut self, key: u64) -> Option<std::fs::File> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_files.get_mut(index)?.take()
    }

    /// Files a spawned child against this task and answers the key that reads
    /// it back — what a `Core\Process\Handle`'s first slot holds.
    ///
    /// The same shape and the same reasoning as [`Ctx::hold_open_file`], which
    /// is the one home of *why* a `Core` handle is a key into a task-owned
    /// table rather than the native thing itself. What this adds is
    /// `rule:core-classes/process-spawn`'s lifetime, and it is stronger than a
    /// descriptor's: a child is not merely released with the task, it is
    /// **killed** with it, because a process that outlived the task that
    /// started it would be work nothing is charged for and nothing can cancel.
    /// [`HeldChild`]'s own `Drop` is that kill, so the table needs no teardown
    /// arm of its own beyond dropping the field.
    ///
    /// **What it spends:** one [`HeldChild`] — a process handle and up to three
    /// pipes — per `spawn` this task performed, *including* the ones it has
    /// since waited for, because a key is never reused. That is
    /// [`Ctx::hold_open_file`]'s trade for its reason: a stale handle reads an
    /// empty slot and throws, where a recycled key would address whatever child
    /// the same slot now holds.
    pub fn hold_spawned_child(&mut self, child: HeldChild) -> u64 {
        self.spawned_children.push(Some(child));
        // The index, one-based, so that a handle slot never holds a key a
        // zeroed value could be mistaken for.
        self.spawned_children.len() as u64
    }

    /// The child `key` names, borrowed for one pipe hand-off or one kill, or
    /// `None` for a key this task never filed and for one a wait is holding.
    pub fn spawned_child_mut(&mut self, key: u64) -> Option<&mut HeldChild> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.spawned_children.get_mut(index)?.as_mut()
    }

    /// Takes the child `key` names out for the length of one off-core wait, or
    /// `None` when another wait is already holding it.
    ///
    /// A borrow will not do here where it does for a read: waiting blocks, so
    /// it happens inside the blocking pool's closure, and that closure owns what
    /// it waits on. The entry is left empty meanwhile, which is what makes a
    /// second `wait` on a handle already inside one a refusal rather than two
    /// threads reaping the same child — and if the task never comes back for
    /// it, the closure's result is dropped and [`HeldChild`]'s `Drop` kills the
    /// child there instead.
    #[must_use]
    pub fn take_spawned_child(&mut self, key: u64) -> Option<HeldChild> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.spawned_children.get_mut(index)?.take()
    }

    /// Puts a child [`Ctx::take_spawned_child`] lent out back in its own slot,
    /// so that the key the program is holding still names it.
    pub fn restore_spawned_child(&mut self, key: u64, child: HeldChild) {
        let Some(index) = key.checked_sub(1).and_then(|at| usize::try_from(at).ok()) else {
            return;
        };
        if let Some(slot) = self.spawned_children.get_mut(index) {
            *slot = Some(child);
        }
    }

    /// How many children this task is still holding, live or reaped — what
    /// `a_spawned_child_is_killed_when_its_task_ends` counts.
    #[must_use]
    pub fn spawned_children(&self) -> usize {
        self.spawned_children.len()
    }

    /// Files an open socket against this request and answers the key that
    /// reads it back — what a `Core\Net\Stream`'s or `Core\Net\Listener`'s one
    /// slot holds.
    ///
    /// The same shape and the same reasoning as [`Ctx::hold_open_file`], which
    /// is where *why* a `Core` handle is a key into a request-owned table is
    /// argued. What this adds is
    /// `rule:core-classes/net-one-api-three-transports`'s lifetime: a socket
    /// closes with the request that opened it, so the table is the request's
    /// and the reactor registration a socket holds is released when this
    /// [`Ctx`] goes. There is no pool, because a connection that outlived its
    /// request would be cross-request state
    /// (`rule:security/no-cross-request-state`).
    ///
    /// One table for both classes rather than one each: what differs between a
    /// connected stream and a listening socket is the type behind the trait
    /// object, which [`HeldSocket::as_any_mut`] hands back at the member that
    /// knows which it asked for. A key names one socket whichever it is, so a
    /// stream's key read as a listener's is a fatal in this crate's caller
    /// rather than a slot that answers plausibly.
    ///
    /// **What it spends:** one `Option<Box<dyn HeldSocket>>` — a pointer pair
    /// — per socket this request opened, *including* the ones it has since
    /// closed, because a key is never reused. That is [`Ctx::hold_open_file`]'s
    /// trade for [`Ctx::hold_open_file`]'s reason: a stale handle reads an
    /// empty slot and throws, where a recycled key would address whatever
    /// socket the same slot now holds.
    pub fn hold_open_socket(&mut self, socket: Box<dyn HeldSocket>) -> u64 {
        self.open_sockets.push(Some(socket));
        // The index, one-based, so that a handle slot never holds a key a
        // zeroed value could be mistaken for.
        self.open_sockets.len() as u64
    }

    /// The socket `key` names, borrowed for one read, write or accept, or
    /// `None` once it has been closed or if it was never this request's.
    ///
    /// [`Ctx::open_connection_mut`]'s shape and its one difference for the same
    /// reason: what comes back is the trait object, because the socket types
    /// are `nvs-host`'s and that crate depends on this one. The caller
    /// downcasts through [`HeldSocket::as_any_mut`].
    pub fn open_socket_mut(&mut self, key: u64) -> Option<&mut dyn HeldSocket> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_sockets.get_mut(index)?.as_deref_mut()
    }

    /// Takes the socket `key` names back out, or `None` when it has already
    /// been taken — a `close` of a handle a previous `close` consumed.
    ///
    /// Dropping what comes back is what closes the descriptor and gives its
    /// reactor registration back, which is why `close` takes rather than marks:
    /// a socket is scarce in the way a descriptor is, and a request that opens
    /// many and closes each as it finishes holds one at a time.
    #[must_use]
    pub fn take_open_socket(&mut self, key: u64) -> Option<Box<dyn HeldSocket>> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_sockets.get_mut(index)?.take()
    }

    /// Files a reply body still arriving against this request and answers the
    /// key that reads it back — what a `Core\Http\Stream`'s body slot holds,
    /// and what the one walk that takes the body carries from then on.
    ///
    /// The same shape and the same reasoning as [`Ctx::hold_open_socket`], and
    /// [`Ctx::hold_started_script`] is the one home of *why* a `Core` handle is
    /// a key into a request-owned table. What this adds is that the key
    /// **moves**: the stream hands it to the reader that takes the body and is
    /// left holding nothing, so "read once" is a property of where the key is
    /// and not only of the refusal that names the member which took it.
    ///
    /// **What it spends:** one `Option<Box<dyn HeldReader>>` — a pointer pair —
    /// per streamed reply this request started, *including* the ones whose walk
    /// has ended, because a key is never reused. The reader behind it is taken
    /// out and dropped at the end of the walk, which is what closes the
    /// connection the rest of the reply would have arrived on; a walk the
    /// program abandons holds that connection until the request ends, which is
    /// [`Ctx::hold_open_file`]'s trade for a forgotten `close` and the same
    /// bound — O(streams this request opened).
    pub fn hold_open_reader(&mut self, reader: Box<dyn HeldReader>) -> u64 {
        self.open_readers.push(Some(reader));
        // The index, one-based, so that a handle slot never holds a key a
        // zeroed value could be mistaken for.
        self.open_readers.len() as u64
    }

    /// The reader `key` names, borrowed for one framing, or `None` once the
    /// walk that held it ended or if it was never this request's.
    ///
    /// [`Ctx::open_socket_mut`]'s shape, for its reason: what comes back is the
    /// trait object, and the caller downcasts through
    /// [`HeldReader::as_any_mut`].
    pub fn open_reader_mut(&mut self, key: u64) -> Option<&mut dyn HeldReader> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_readers.get_mut(index)?.as_deref_mut()
    }

    /// Takes the reader `key` names back out, or `None` where a walk has
    /// already taken it.
    ///
    /// Dropping what comes back closes the connection the rest of the body
    /// would have arrived on, which is why a walk that reaches the end of a
    /// body takes rather than marks: the socket is scarce in the way a
    /// descriptor is, and a request reading many replies in turn holds one.
    #[must_use]
    pub fn take_open_reader(&mut self, key: u64) -> Option<Box<dyn HeldReader>> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_readers.get_mut(index)?.take()
    }

    /// Files an open database connection against this request and answers the
    /// key that reads it back — what a `Core\Db\Connection`'s first slot holds.
    ///
    /// The same shape and the same reasoning as [`Ctx::hold_open_file`], and
    /// [`Ctx::hold_started_script`] is the one home of *why* a `Core` handle is
    /// a key into a request-owned table. What this adds is `memo`, which is
    /// `rule:core-classes/db-connection-is-named`'s memoization key —
    /// the block's name for `Core\Db::connect`, and `None` for a
    /// `{shared: false}` call, which is exactly what "bypasses memoization"
    /// means: an entry no [`Ctx::memoized_connection`] lookup can match. The
    /// key is held beside the connection rather than in a map of its own
    /// because a request opens a handful of connections at most, so a linear
    /// scan is the whole lookup and an empty request pays no allocation for it.
    ///
    /// `lease` is the other half, and it is § 13's: it is the slot this
    /// connection is live under, and it names the pool the connection rejoins
    /// when the request ends instead of being closed. It is **not** the
    /// memoization key even though `connect` computes both from the block's
    /// name — a `{shared: false}` call is `None` for `memo` and still carries a
    /// lease, because what that option bypasses is memoization within the
    /// request and never pooling across requests. `None` is a connection closed
    /// with the request and counted against nothing: an embedder's.
    ///
    /// **What it spends:** one connection — a socket, a TLS session and its
    /// statement cache — per distinct `connect` a request performs, and at
    /// teardown it is handed to [`crate::pool`] rather than closed, under that
    /// module's bounds. A key is never reused within a request.
    pub fn hold_open_connection(
        &mut self,
        memo: Option<String>,
        lease: Option<crate::pool::Lease>,
        connection: Box<dyn HeldConnection>,
    ) -> u64 {
        self.open_connections.push(OpenConnection {
            memo,
            lease,
            connection: Some(connection),
            enqueued_under: None,
        });
        // The index, one-based, so that a handle slot never holds a key a
        // zeroed value could be mistaken for.
        self.open_connections.len() as u64
    }

    /// The connection `key` names, borrowed for one statement, or `None` for a
    /// key this request never filed.
    ///
    /// [`Ctx::open_file_mut`]'s shape, and one difference that is
    /// [`HeldConnection`]'s whole reason: what comes back is the trait object
    /// rather than a driver's own type, because
    /// `rule:core-classes/db-crate-boundary` has `nvs-db` depending on this crate and naming `nvs_db::Connection`
    /// here would close a cycle. The caller that knows which crate opened it
    /// gets its own type back through
    /// [`HeldConnection::as_any_mut`](crate::HeldConnection::as_any_mut) — one
    /// downcast, at the one place a statement is written.
    ///
    /// `None` covers both a key this request never filed and one whose
    /// connection [`Ctx::close_open_connection`] has already released;
    /// [`Ctx::connection_is_filed`] is what tells those two apart, and the
    /// caller needs to, because they are a paste error in the caller's own
    /// crate and a program's `close`-then-use respectively.
    pub fn open_connection_mut(&mut self, key: u64) -> Option<&mut dyn HeldConnection> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_connections
            .get_mut(index)?
            .connection
            .as_deref_mut()
    }

    /// Whether `key` names an entry this request filed at all, open or closed.
    #[must_use]
    pub fn connection_is_filed(&self, key: u64) -> bool {
        key.checked_sub(1)
            .and_then(|index| usize::try_from(index).ok())
            .is_some_and(|index| index < self.open_connections.len())
    }

    /// Releases the connection `key` names, answering whether it was still
    /// open — spec § 18's `Core\Db\Connection::close`, which is the one thing
    /// that ends a connection's life before the request's.
    ///
    /// It is the teardown loop in [`Ctx::drop`] for exactly one entry, and
    /// deliberately the same two lines: `rule:security/db-pool-reset-is-a-boundary`'s release is where a
    /// leased connection goes, and an unleased one — an embedder's — is
    /// dropped. A `close` therefore returns a connection to this core's pool
    /// *earlier* than the request would have, which is the whole reason a
    /// program that is finished with one writes it.
    ///
    /// **Idempotent, and it is the only member that is.** A second `close` has
    /// nothing to release and answers `false`; every other member on a closed
    /// handle refuses, because a statement on a connection that is gone is a
    /// mistake a program can only have made on purpose. The entry itself stays,
    /// per [`OpenConnection::connection`].
    pub fn close_open_connection(&mut self, key: u64) -> bool {
        let Some(index) = key.checked_sub(1).and_then(|at| usize::try_from(at).ok()) else {
            return false;
        };
        let Some(held) = self.open_connections.get_mut(index) else {
            return false;
        };
        let Some(connection) = held.connection.take() else {
            return false;
        };
        match held.lease.take() {
            Some(lease) => crate::pool::release(lease, std::time::Instant::now(), connection),
            None => drop(connection),
        }
        true
    }

    /// The key of the connection this request already opened under `memo`, or
    /// `None` for a name it has not reached yet — § 2's memoization, asked.
    ///
    /// **A closed entry does not answer.** § 2 memoizes so that a second
    /// `connect("main")` is the same *connection*, and a handle whose
    /// connection has been released is not one — so the name is free again and
    /// the next `connect` opens and files a second entry, which is what a
    /// program that wrote `close` asked for.
    #[must_use]
    pub fn memoized_connection(&self, memo: &str) -> Option<u64> {
        self.open_connections
            .iter()
            .position(|held| held.memo.as_deref() == Some(memo) && held.connection.is_some())
            .map(|index| index as u64 + 1)
    }

    /// Records that a job was enqueued on connection `key` while `open`
    /// transaction levels were open on it, counted from 1 for the outermost.
    ///
    /// The job is durable only when the outermost level commits, and whoever
    /// announces new work has to wait for that: a worker told earlier would
    /// look for a row no other connection can see yet.
    /// [`Ctx::transaction_level_closed`] is where each level's ending is
    /// judged.
    ///
    /// **One number per connection, however many jobs a transaction
    /// enqueues**, and the shallowest level is the one kept. A rollback undoes
    /// every enqueue made at its own level or inside it, so an enqueue at a
    /// shallower level survives whatever a deeper one does, and it is the one
    /// the outermost commit is owed for.
    ///
    /// A key this request never filed records nothing.
    ///
    /// **What it spends:** one word per connection a request has open.
    pub fn note_enqueued_under(&mut self, key: u64, open: u32) {
        if let Some(held) = self.open_connection_entry(key) {
            held.enqueued_under = Some(held.enqueued_under.map_or(open, |had| had.min(open)));
        }
    }

    /// Judges what [`Ctx::note_enqueued_under`] recorded against transaction
    /// `level` of connection `key` closing, and answers whether an enqueue has
    /// just become durable.
    ///
    /// `level` is counted as that method counts it, so the outermost
    /// transaction is 1. A commit of a nested level hands what was recorded
    /// inside it to the level around it, a commit of the outermost one answers
    /// `true` and clears the record, and a rollback clears whatever was
    /// recorded at its level or deeper and answers `false`. An enqueue recorded
    /// under a shallower level than the one closing is left as it is.
    pub fn transaction_level_closed(&mut self, key: u64, level: u32, committed: bool) -> bool {
        let Some(held) = self.open_connection_entry(key) else {
            return false;
        };
        let Some(under) = held.enqueued_under else {
            return false;
        };
        if under < level {
            return false;
        }
        let around = level.saturating_sub(1);
        held.enqueued_under = (committed && around > 0).then_some(around);
        committed && around == 0
    }

    /// The entry `key` names, open or closed, or `None` for a key this request
    /// never filed.
    fn open_connection_entry(&mut self, key: u64) -> Option<&mut OpenConnection> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_connections.get_mut(index)
    }

    /// Records a directory `Core\IO::temporaryDir` has just created for this
    /// script — `rule:core-classes/temporary-dir-sweep`'s per-script list, written by
    /// [`crate::capability::temp_dir`](crate::capability::temp_dir) and by
    /// nothing else.
    ///
    /// Recorded **after** the directory exists, so the list is what the runtime
    /// actually created rather than what it intended to: a call the capability
    /// check refused, or one the operating system did, leaves nothing on disk
    /// and so leaves nothing here for § 3's sweep to fail to delete.
    ///
    /// No handle and no key, unlike the tables above. A directory's name
    /// is the whole of it, a program never asks this context for one back, and
    /// a program that removes its own directory has reached the goal state early
    /// (§ 3) — so there is nothing to take out of the middle of the list and the
    /// entry simply stays until the sweep drains all of them.
    ///
    /// **What it spends:** one `PathBuf` per `temporaryDir` call this script
    /// made — O(directories created), request-local, released with the request
    /// and charged to its memory limit, which is the bound
    /// `rule:programs/memory-priority` asks for.
    /// A script that never calls the member allocates nothing at all.
    pub fn track_temporary_dir(&mut self, path: std::path::PathBuf) {
        self.temporary_dirs.push(path);
    }

    /// The directories this script has been handed, in the order it asked for
    /// them.
    #[must_use]
    pub fn temporary_dirs(&self) -> &[std::path::PathBuf] {
        &self.temporary_dirs
    }

    /// Takes the whole list, leaving this context with none — what § 3's sweep
    /// calls once, after the last user code.
    ///
    /// Draining rather than borrowing, for two reasons the sweep depends on. It
    /// needs the paths while holding the context mutably, to log a refusal
    /// through the same record `Core\Log` writes; and the sweep must be
    /// idempotent, because a request that dies mid-flight is swept by whichever
    /// of its endings the worker reaches first and the ordinary end may follow.
    /// An emptied list makes the second call a no-op rather than a second round
    /// of deletions of paths that are already gone.
    #[must_use]
    pub fn take_temporary_dirs(&mut self) -> Vec<std::path::PathBuf> {
        std::mem::take(&mut self.temporary_dirs)
    }
}

#[cfg(test)]
mod tests {
    use super::{Ctx, HeldConnection};

    /// A connection that is nothing but an entry in the table: what is under
    /// test is what the request records beside it.
    #[derive(Debug)]
    struct Filed;

    impl HeldConnection for Filed {
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
            self
        }
    }

    /// A request holding one connection, and that connection's key.
    fn holding_one() -> (Ctx, u64) {
        let mut ctx = Ctx::stdout();
        let key = ctx.hold_open_connection(None, None, Box::new(Filed));
        (ctx, key)
    }

    /// The outermost commit is the one that makes an enqueue durable, and it
    /// says so once.
    #[test]
    fn the_outermost_commit_answers_for_an_enqueue_made_under_it() {
        let (mut ctx, key) = holding_one();
        ctx.note_enqueued_under(key, 1);
        assert!(
            ctx.transaction_level_closed(key, 1, true),
            "the commit of the level a job was enqueued under did not answer for it"
        );
        assert!(
            !ctx.transaction_level_closed(key, 1, true),
            "a later transaction's commit answered for an enqueue the one before it had settled"
        );
    }

    /// A rollback undoes the enqueue, so nothing is owed for it afterwards.
    #[test]
    fn a_rollback_answers_nothing_and_leaves_nothing_owed() {
        let (mut ctx, key) = holding_one();
        ctx.note_enqueued_under(key, 1);
        assert!(
            !ctx.transaction_level_closed(key, 1, false),
            "a rollback answered for the job it undid"
        );
        assert!(
            !ctx.transaction_level_closed(key, 1, true),
            "a job a rollback undid was answered for by the next transaction's commit"
        );
    }

    /// A nested level's commit hands its enqueue to the level around it, which
    /// is the one whose commit answers.
    #[test]
    fn a_nested_commit_hands_its_enqueue_to_the_level_around_it() {
        let (mut ctx, key) = holding_one();
        ctx.note_enqueued_under(key, 2);
        assert!(
            !ctx.transaction_level_closed(key, 2, true),
            "a nested level's commit answered while the outermost one was still open"
        );
        assert!(
            ctx.transaction_level_closed(key, 1, true),
            "the outermost commit did not answer for a job a nested level handed it"
        );
    }

    /// A nested rollback takes its own enqueue with it and leaves the outer
    /// level's alone.
    #[test]
    fn a_nested_rollback_undoes_its_own_enqueue_and_no_other() {
        let (mut ctx, key) = holding_one();
        ctx.note_enqueued_under(key, 2);
        assert!(!ctx.transaction_level_closed(key, 2, false));
        assert!(
            !ctx.transaction_level_closed(key, 1, true),
            "the outermost commit answered for a job its nested level had rolled back"
        );

        ctx.note_enqueued_under(key, 1);
        ctx.note_enqueued_under(key, 2);
        assert!(!ctx.transaction_level_closed(key, 2, false));
        assert!(
            ctx.transaction_level_closed(key, 1, true),
            "a nested rollback took the outer level's own enqueue with it"
        );
    }

    /// An outer rollback undoes what a nested level had already committed into
    /// it.
    #[test]
    fn an_outer_rollback_undoes_what_a_nested_commit_handed_it() {
        let (mut ctx, key) = holding_one();
        ctx.note_enqueued_under(key, 2);
        assert!(!ctx.transaction_level_closed(key, 2, true));
        assert!(
            !ctx.transaction_level_closed(key, 1, false),
            "an outer rollback answered for a job it undid"
        );
        assert!(!ctx.transaction_level_closed(key, 1, true));
    }

    /// What one connection recorded is not another's, and a key nothing was
    /// filed under records nothing at all.
    #[test]
    fn an_enqueue_is_recorded_against_its_own_connection() {
        let (mut ctx, key) = holding_one();
        let other = ctx.hold_open_connection(None, None, Box::new(Filed));
        ctx.note_enqueued_under(key, 1);
        ctx.note_enqueued_under(other + 1, 1);
        assert!(
            !ctx.transaction_level_closed(other, 1, true),
            "one connection's commit answered for a job enqueued on another"
        );
        assert!(!ctx.transaction_level_closed(other + 1, 1, true));
        assert!(ctx.transaction_level_closed(key, 1, true));
    }
}
