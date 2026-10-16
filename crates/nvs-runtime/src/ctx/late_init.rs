//! `rule:classes/late-init-initializes-on-the-first-read`'s in-flight tables:
//! which `initProperty` calls are running, for one task and for the request.
//!
//! Two tables, because two questions are asked. A read inside its own
//! `initProperty` throws, and a write inside that call is plain, so both ask
//! about *this task's* calls: [`Ctx::late_inits`], a `Vec` on the task's own
//! context. A read from another task of the same request waits for the call
//! instead, so it asks about *every* task's calls: [`Table`], one per request,
//! which [`Ctx::child`] shares with each task. An isolate shares no object, so
//! it gets a table of its own.
//!
//! Both are keyed by the object's address and the slot. An entry lives only
//! while its call runs, and a waiter only while its task is parked, so the
//! tables cost O(in-flight initializations) and nothing per slot or per object.
//!
//! **What it spends** (`rule:programs/memory-priority`): the shared table is one
//! small allocation per request that calls `initProperty` or starts a task, and
//! nothing for any other request.
//!
//! The table is not [`super::TreeState`]: that one is shared across threads and
//! with isolates, and a [`Waker`] is neither `Send` nor anything an isolate may
//! reach.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::Ctx;
use crate::Fault;
use crate::host::{self, Waker, Woken};

/// The object's address and the slot an `initProperty` call initializes.
type Key = (usize, usize);

/// Every `initProperty` call running in one request, with the reads parked on
/// each.
#[derive(Default)]
pub(super) struct Table {
    running: RefCell<Vec<Running>>,
    /// The next waiter's number, so a parked read removes its own entry and no
    /// other.
    next_ticket: Cell<u64>,
}

/// The keys running and how many reads wait on each. A [`Waker`] has no
/// `Debug`, so this is written by hand.
impl std::fmt::Debug for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(
                self.running
                    .borrow()
                    .iter()
                    .map(|entry| (entry.key, entry.waiters.len())),
            )
            .finish()
    }
}

struct Running {
    key: Key,
    waiters: Vec<Waiter>,
}

struct Waiter {
    ticket: u64,
    settled: Rc<Cell<Option<Settled>>>,
    wake: Waker,
}

/// How a running `initProperty` call ended, as a read parked on it learns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settled {
    /// It returned. The parked read reads the slot again and calls nothing.
    Returned,
    /// It threw or was cancelled, so the property is still uninitialized. The
    /// parked read looks again, and calls `initProperty` itself when no other
    /// task has started it.
    Failed,
}

/// What [`Ctx::begin_late_init`] found.
#[derive(Debug)]
pub enum LateInit {
    /// No call for the key was running, and this task's call is now the one
    /// running. Hand it back to [`Ctx::end_late_init`].
    Started(LateInitCall),
    /// This task is already inside `initProperty` for the key.
    Recursive,
    /// Another task of this request is inside `initProperty` for the key.
    /// [`Ctx::wait_late_init`] parks until it ends.
    Running,
}

/// One running `initProperty` call, entered in the request's [`Table`].
///
/// Dropping it removes the entry and wakes every read parked on it, which is
/// what happens when a cancelled task's stack is unwound through the call.
/// Those reads then learn [`Settled::Failed`].
#[derive(Debug)]
pub struct LateInitCall {
    table: Rc<Table>,
    key: Key,
    settled: Settled,
}

impl Drop for LateInitCall {
    fn drop(&mut self) {
        let waiters = {
            let mut running = self.table.running.borrow_mut();
            match running.iter().position(|entry| entry.key == self.key) {
                Some(at) => running.swap_remove(at).waiters,
                None => Vec::new(),
            }
        };
        for waiter in waiters {
            waiter.settled.set(Some(self.settled));
            (waiter.wake)();
        }
    }
}

/// A read parked on a running call. Dropping it removes the read's waiter if
/// the call has not already taken it, which covers a cancelled read whose
/// stack is unwound.
struct Parked<'t> {
    table: &'t Table,
    key: Key,
    ticket: u64,
}

impl Drop for Parked<'_> {
    fn drop(&mut self) {
        let mut running = self.table.running.borrow_mut();
        if let Some(entry) = running.iter_mut().find(|entry| entry.key == self.key) {
            entry.waiters.retain(|waiter| waiter.ticket != self.ticket);
        }
    }
}

impl Ctx {
    /// Records that this task starts an `initProperty` call for `key`, unless a
    /// call for the same key is already running in this task or in another
    /// task of this request.
    pub fn begin_late_init(&mut self, key: Key) -> LateInit {
        if self.late_inits.contains(&key) {
            return LateInit::Recursive;
        }
        let table = Rc::clone(self.late_table());
        if table.running.borrow().iter().any(|entry| entry.key == key) {
            return LateInit::Running;
        }
        table.running.borrow_mut().push(Running {
            key,
            waiters: Vec::new(),
        });
        self.late_inits.push(key);
        LateInit::Started(LateInitCall {
            table,
            key,
            settled: Settled::Failed,
        })
    }

    /// Records that the call [`Self::begin_late_init`] started has ended as
    /// `settled`, and wakes every read parked on it.
    pub fn end_late_init(&mut self, mut call: LateInitCall, settled: Settled) {
        if let Some(at) = self.late_inits.iter().rposition(|held| *held == call.key) {
            self.late_inits.remove(at);
        }
        call.settled = settled;
    }

    /// Parks this task until the call another task runs for `key` ends, and
    /// returns how it ended. `None` means the task woke for another reason, or
    /// the call had already ended, and the caller looks again.
    ///
    /// # Errors
    ///
    /// [`Ctx::cancel`]'s fault when the task is cancelled while it waits, and a
    /// [`Fault::fatal`] when there is no scheduler on this thread to wait on.
    pub fn wait_late_init(&mut self, key: Key) -> Result<Option<Settled>, Fault> {
        let table = Rc::clone(self.late_table());
        let Some(wake) = host::with_current(|host| host.waker()).flatten() else {
            return Err(Fault::fatal(
                "a read waits for another task's `initProperty`, and there is no scheduler on \
                 this thread to wait on",
            ));
        };
        let ticket = table.next_ticket.get();
        table.next_ticket.set(ticket.wrapping_add(1));
        let settled = Rc::new(Cell::new(None));
        {
            let mut running = table.running.borrow_mut();
            let Some(entry) = running.iter_mut().find(|entry| entry.key == key) else {
                return Ok(None);
            };
            entry.waiters.push(Waiter {
                ticket,
                settled: Rc::clone(&settled),
                wake,
            });
        }
        let parked = Parked {
            table: &table,
            key,
            ticket,
        };
        let woken = host::with_current(|host| host.park(None));
        drop(parked);
        if matches!(woken, Some(Woken::Cancelled)) {
            return Err(self.cancel());
        }
        Ok(settled.get())
    }

    /// Whether an `initProperty` call is running in this task, which is what
    /// makes a write that initializes a `lateinit` property plain. A call in
    /// another task of the request does not count.
    #[must_use]
    pub fn late_init_running(&self) -> bool {
        !self.late_inits.is_empty()
    }

    /// The request's [`Table`], made the first time anything asks for it.
    pub(super) fn late_table(&self) -> &Rc<Table> {
        self.late_table.get_or_init(Rc::default)
    }
}
