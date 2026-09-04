//! What a request holds open, and gives back when it ends.
//!
//! Started scripts, open files and open connections: three tables of handles a
//! member takes out and a key it hands back, so that a value in Novis code is a
//! number rather than a pointer and a handle awaited twice reads an empty slot
//! rather than another request's resource.
//!
//! [`HeldConnection`] is the trait that lets this crate hold a
//! [ADR 0132](/docs/adr/0132-database-driver-shape.md) driver's connection
//! without depending on the driver — the dependency runs the other way, so the
//! field is a `dyn Trait` and the only method on it is the downcast a holder
//! needs to get its own type back.

use super::*;

/// A database connection a request is holding open — `nvs_db`'s `Connection`,
/// and the seam that lets a [`Ctx`] hold one without naming it.
///
/// Declared here for [`Running`](crate::host::Running)'s reason and one more.
/// A `Core\Db\Connection` is an object with no native drop, so the connection
/// itself is a key into a table the request owns
/// ([`Ctx::hold_open_connection`]); the table has to live in this crate,
/// because this is the crate that learns when a request ends. And the edge
/// cannot run the other way: [ADR 0132](/docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)
/// § 1 has `nvs-db` depending on this crate, so a field typed
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
    /// [ADR 0067](/docs/adr/0067-core-db.md) § 13's reset consumes the
    /// connection so that a failed one cannot be handed back.
    ///
    /// No default body: it would have to coerce `Self` to `dyn Any`, which a
    /// trait's own body cannot do without knowing `Self: Sized`. Every impl is
    /// `self`, one line.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;

    /// Whether this connection may rejoin the core's pool at teardown —
    /// [ADR 0067](/docs/adr/0067-core-db.md) § 13's release gate, asked
    /// of the driver because only the driver knows where its wire is.
    ///
    /// **The default is `false`**, which is § 13's "a driver with no reset
    /// primitive is not poolable at all" written as the answer a driver gets
    /// for saying nothing. A driver that has not decided is one whose
    /// connections are closed with the request, which is the behaviour that
    /// existed before there was a pool.
    ///
    /// This is a state question and never an I/O one: it is asked from inside
    /// [`Drop`], where nothing may wait. The reset itself is the acquiring
    /// request's, and [`crate::pool`]'s module doc owns why.
    fn is_poolable(&self) -> bool {
        false
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
    pub(super) lease: Option<crate::pool::Lease>,
    /// The connection, held as the trait object for the reason
    /// [`HeldConnection`]'s own doc gives.
    pub(super) connection: Box<dyn HeldConnection>,
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
    /// which [ADR 0004](/docs/adr/0004-memory-for-simplicity.md) calls
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
    /// [ADR 0004](/docs/adr/0004-memory-for-simplicity.md)'s ordering
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

    /// Files an open database connection against this request and answers the
    /// key that reads it back — what a `Core\Db\Connection`'s first slot holds.
    ///
    /// The same shape and the same reasoning as [`Ctx::hold_open_file`], and
    /// [`Ctx::hold_started_script`] is the one home of *why* a `Core` handle is
    /// a key into a request-owned table. What this adds is `memo`, which is
    /// [ADR 0067](/docs/adr/0067-core-db.md) § 2's memoization key —
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
            connection,
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
    /// [ADR 0132](/docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)
    /// § 1 has `nvs-db` depending on this crate and naming `nvs_db::Connection`
    /// here would close a cycle. The caller that knows which crate opened it
    /// gets its own type back through
    /// [`HeldConnection::as_any_mut`](crate::HeldConnection::as_any_mut) — one
    /// downcast, at the one place a statement is written.
    ///
    /// There is no `take_open_connection` beside it and there is not meant to
    /// be one yet: spec § 18's `Core\Db\Connection::close` is what would take a
    /// connection back out, and until it exists every connection this request
    /// filed leaves through [`Drop`], which is the one place ADR 0067 § 13's
    /// release is written.
    pub fn open_connection_mut(&mut self, key: u64) -> Option<&mut dyn HeldConnection> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_connections
            .get_mut(index)
            .map(|held| &mut *held.connection)
    }

    /// The key of the connection this request already opened under `memo`, or
    /// `None` for a name it has not reached yet — § 2's memoization, asked.
    #[must_use]
    pub fn memoized_connection(&self, memo: &str) -> Option<u64> {
        self.open_connections
            .iter()
            .position(|held| held.memo.as_deref() == Some(memo))
            .map(|index| index as u64 + 1)
    }
}
