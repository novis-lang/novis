//! [ADR 0067 § 7](/docs/decisions/0067.md): a transaction runs a
//! callable, and what happens when it conflicts.
//!
//! The callable being the whole interface is what makes commit and rollback this
//! module's decision rather than the program's — `rollBack` is the one explicit
//! exit and it is still inside the callable. A retryable failure is taken again
//! under the exponential, jittered, capped wait [`retry_backoff`] renders, and
//! [`Attempts`] is why that rule can be asserted at all: a case scripts the
//! conflict instead of provoking a real deadlock between two connections.

use super::*;

/// The [`ISOLATION`] case an `{isolation: …}` option arrived as, or `None` for
/// the call that left it out.
///
/// **The two rosters are matched by ordinal and written out**, not cast:
/// [`ISOLATION`] states § 7's order for the program and [`nvs_db::Isolation`]
/// states it again for the driver, in different crates for different readers,
/// and a cast between them would answer the wrong level the first time either
/// gained a case. `Core\Cli`'s `stream_of` is the same judgement.
///
/// `None` is the option's declared default and not a failure: § 7's absent
/// `isolation` runs at the level the connection already has, which
/// [`nvs_db::PgConn::begin`] renders by leaving the clause off the `BEGIN`.
pub(super) fn isolation_of(
    value: &Value,
    member: &str,
) -> Result<Option<nvs_db::Isolation>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    match value.as_int() {
        Some(0) => Ok(Some(nvs_db::Isolation::ReadUncommitted)),
        Some(1) => Ok(Some(nvs_db::Isolation::ReadCommitted)),
        Some(2) => Ok(Some(nvs_db::Isolation::RepeatableRead)),
        Some(3) => Ok(Some(nvs_db::Isolation::Snapshot)),
        Some(4) => Ok(Some(nvs_db::Isolation::Serializable)),
        // Unreachable from source: the option is `CoreTy::Enum(ISOLATION_NAME)`
        // in [`TRANSACTION_OPTIONS`], so anything that is not one of the five
        // cases is `E0401` at the checker and compiled code writes the ordinal
        // itself. `Core\Arr::sort`'s `order` states the same reasoning in full.
        _ => Err(Fault::fatal(format!(
            "{member} expected a `{ISOLATION_NAME}` case for `isolation`, got tag {} value {:?}",
            value.tag_byte(),
            value.as_int()
        ))),
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Queryable::transaction(callable $fn, {isolation?, readOnly?,
    /// retries?}): T` — `rule:core-classes/db-transactions`'s whole shape, and the only way to open a
    /// transaction on this surface.
    ///
    /// **The callable form is what removes the failure mode**, which § 7 argues
    /// and this body implements: there is no point between the `BEGIN` and the
    /// `COMMIT` at which a program can walk away, because the scope is a call
    /// and an early `return` inside it is still a return *through* here. With
    /// no destructors there is nothing an object-scoped transaction could hook
    /// its rollback to, so the callable is not the tidier of two options — it is
    /// the one that can be made to hold.
    ///
    /// **Committing is what returning does, and there is no member for it.**
    /// The three outcomes are decided here rather than by the callable: it
    /// returned and nothing asked for a rollback, so the work commits and its
    /// answer is this call's; it threw, so the work rolls back and its
    /// exception travels on unchanged; or it recorded a reason through
    /// [`nvs_core_db_transaction_roll_back`], so the work rolls back and
    /// `Core\Db\RolledBack` is raised here.
    ///
    /// **The third case is read off the transaction and not off the throw**,
    /// which is the point of § 7's flag: an intervening `catch (Throwable)`
    /// swallows the signal, the callable returns normally, and this frame still
    /// refuses to commit. That is the guarantee Doctrine's `setRollbackOnly`
    /// asks every layer to cooperate on.
    ///
    /// **A nested call is a savepoint**, and nothing here says so: the depth
    /// lives on the connection and [`nvs_db::PgConn::begin`] picks the command
    /// from it, so a library that wraps its own writes composes with a caller's
    /// transaction without either of them knowing.
    ///
    /// **`isolation` and `readOnly` are handed straight to that same call**,
    /// which is where both the rendering and the one refusal live: a nested
    /// call carrying either is an `InvalidInput` there and so a `LogicError`
    /// here, because PostgreSQL settles both for the whole transaction and
    /// running the callable at the outer one's level would be quietly weaker
    /// than what its author wrote. Reading the two here and deciding nothing
    /// with them is deliberate — the level a given backend can offer is the
    /// driver's question, and [`TRANSACTION_OPTIONS`] owns what their defaults
    /// mean.
    ///
    /// **`{retries: n}` goes around the whole block and not inside it.** Each
    /// attempt gets its own `BEGIN` and its own scope object, because the one
    /// above is closed and discarded on every path already — a re-run that
    /// reused either would be handing the callable a `$tx` that is refusing.
    /// **Either conflict re-runs it**: the commit's own refusal, and one a
    /// statement inside the callable raised, which arrives as a pending
    /// `Core\Db\DbError` instead and is read through
    /// [`nvs_runtime::Ctx::pending_slot`]. [`wait_between_attempts`] is the
    /// wait § 7 puts between two of them. The loop itself is
    /// [`transacted`], which is handed its connection rather than reading one
    /// back out of the context — see [`Attempts`] for why that seam is where
    /// it is.
    ///
    /// **Rolling back after a throw discards its own failure.** The exception
    /// the callable raised is what the request is about, and a connection whose
    /// `ROLLBACK` was refused is one § 13's reset destroys rather than pools —
    /// so replacing the program's exception with the driver's would lose the
    /// only half a caller can act on.
    fn nvs_core_db_connection_transaction(ctx, args: [5]) {
        let (key, block) = handle_of(args[0], "transaction")?;
        let isolation = isolation_of(&args[ISOLATION_ARG], TRANSACTION_MEMBER)?;
        // Unreachable from source for `connect`'s reason: the option is
        // declared `bool` and defaults to one, so the slot is never anything
        // else.
        let read_only = args[READ_ONLY_ARG].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "{TRANSACTION_MEMBER} expected a `bool` for `readOnly`, got tag {}",
                args[READ_ONLY_ARG].tag_byte()
            ))
        })?;
        // As `readOnly`: the option is declared `uint` and defaults to 0.
        let retries = args[RETRIES_ARG].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{TRANSACTION_MEMBER} expected a `uint` for `retries`, got tag {}",
                args[RETRIES_ARG].tag_byte()
            ))
        })?;

        transacted(
            ctx,
            &mut Filed { key },
            &Attempted {
                key,
                block,
                callable: args[1],
                isolation,
                read_only,
                retries,
            },
        )
    }
}

/// § 7's four questions to the connection an attempt runs on, as something
/// [`transacted`] is *handed* rather than reads back out of the context.
///
/// **The retry loop is the half of § 7 no driver implements.** A `BEGIN`, a
/// `SAVEPOINT` and a `ROLLBACK` are `nvs-db`'s and are asserted there; nothing
/// in a driver is ever asked to *re-run* anything, so the loop is the one § 7
/// behaviour no `-p nvs-db` case can reach. This trait is what makes it
/// reachable from a `-p nvs-stdlib` one instead: [`Filed`] is the only
/// implementation the runtime ever builds, and a test scripts the
/// conflict-then-commit pair a real deadlock produces without a server in front
/// of it — which it could not otherwise do, [`Transacting`]'s two arms both
/// being connections no test can construct.
///
/// **Every method is handed the context rather than borrowing out of it.** The
/// loop calls Novis code between the `BEGIN` and the `COMMIT`, and that needs
/// the same `&mut Ctx` the connection is filed in, so a borrow held across the
/// callable cannot exist. That is why this is four questions asked one at a
/// time rather than one that answers a [`Transacting`], and it is the same
/// reason the sites below re-read the connection at each of them.
///
/// **Two error channels, which is what the nested `Result` is.** The outer
/// failure is *this request has no such connection* — already a [`Fault`],
/// from [`transacting`] — and the inner one is the server's own refusal, which
/// § 7's retry rule reads a [`nvs_db::DbErrorKind`] off through
/// [`nvs_db::ServerError`] and which only [`std::io::Error`] still carries.
/// Collapsing the two would leave the loop deciding a retry off a rendered
/// message.
pub(super) trait Attempts {
    /// How many of § 7's levels are open, before this attempt's `BEGIN`.
    ///
    /// # Errors
    ///
    /// As [`transacting`].
    fn depth(&mut self, ctx: &mut nvs_runtime::Ctx) -> Result<u32, Fault>;

    /// § 7's outermost `BEGIN`, or the `SAVEPOINT` a nested call opens.
    ///
    /// # Errors
    ///
    /// As [`transacting`]; the driver's own refusal is the inner `Err`.
    fn begin(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
        isolation: Option<nvs_db::Isolation>,
        read_only: bool,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault>;

    /// § 7's `COMMIT`, or the release that closes a nested level.
    ///
    /// # Errors
    ///
    /// As [`Self::begin`].
    fn commit(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault>;

    /// § 7's `ROLLBACK`, or the undo of a nested level.
    ///
    /// # Errors
    ///
    /// As [`Self::begin`].
    fn roll_back(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault>;
}

/// [`Attempts`] over the connection this request has filed under `key` — the
/// only implementation outside this module's own tests.
///
/// **One field, since [`transacting`] became total.** It carried the
/// `[db.<name>]` block as well for as long as there was a driver to turn away
/// by name; [`Attempted`] still holds one, because § 8's refusals are thrown
/// about a *statement* and name the connection they ran on.
pub(super) struct Filed {
    /// § 2's key, as [`handle_of`] read it off the receiver.
    key: u64,
}

impl Attempts for Filed {
    fn depth(&mut self, ctx: &mut nvs_runtime::Ctx) -> Result<u32, Fault> {
        Ok(transacting(ctx, self.key, TRANSACTION_MEMBER)?.depth())
    }

    fn begin(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
        isolation: Option<nvs_db::Isolation>,
        read_only: bool,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
        Ok(transacting(ctx, self.key, TRANSACTION_MEMBER)?.begin(isolation, read_only))
    }

    fn commit(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
        Ok(transacting(ctx, self.key, TRANSACTION_MEMBER)?.commit())
    }

    fn roll_back(
        &mut self,
        ctx: &mut nvs_runtime::Ctx,
    ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
        Ok(transacting(ctx, self.key, TRANSACTION_MEMBER)?.roll_back())
    }
}

/// § 7's call, as [`nvs_core_db_connection_transaction`] read it off its
/// arguments.
///
/// One struct rather than five more parameters on [`transacted`], which takes
/// the connection separately: splitting those two halves is what the hoist is
/// for, and a seven-argument function would be the shape it was written to
/// avoid. Neither [`Value`] carries a reference of its own — both are the
/// helper's own arguments, live for the whole call.
pub(super) struct Attempted {
    /// § 2's key, as the scope object carries it on to a delegated member.
    key: u64,
    /// The `[db.<name>]` block this transaction refuses under.
    block: Value,
    /// § 7's `$fn`, run once per attempt.
    callable: Value,
    /// § 7's `{isolation}`, decided by the driver and refused when nested.
    isolation: Option<nvs_db::Isolation>,
    /// § 7's `{readOnly}`, on the same terms.
    read_only: bool,
    /// § 7's `{retries: n}` — how many re-runs a conflict may still have.
    retries: u64,
}

/// The first rung of § 7's backoff, and the base every rung after it doubles
/// from.
///
/// § 7 names no number and `{retries: n}` has no key for one, which is the
/// right shape rather than an omission: the wait is not spent letting a lock
/// clear. A server reports a deadlock or a serialization failure only once it
/// has already *resolved* the conflict — PostgreSQL detects a deadlock by
/// breaking it — so what the wait buys is the de-correlation of two requests
/// that conflicted with each other and would otherwise re-run into each other
/// on the same schedule. Ten milliseconds is a spread wide enough for that and
/// narrow enough that a request retried once does not notice it.
pub(super) const RETRY_BACKOFF: std::time::Duration = std::time::Duration::from_millis(10);

/// The ceiling one rung may reach, however many retries were asked for.
///
/// `retries` is a `uint`, so the rung is capped here or a program naming sixty
/// of them draws its wait out of a range measured in years. A second is already
/// past the point where waiting longer buys anything — it is the whole latency
/// budget of the request the retry exists to save.
pub(super) const RETRY_BACKOFF_CAP: std::time::Duration = std::time::Duration::from_secs(1);

/// § 7's wait before the retry with `taken` of them already spent: full jitter
/// over an exponential rung, capped at [`RETRY_BACKOFF_CAP`].
///
/// **Full jitter — uniform in `[0, base × 2^taken]` — and not the rung
/// itself**, which is `crate::http::transport`'s shape for
/// `rule:http-server/retry-is-opt-in-jittered-and-closed` and
/// holds here for a sharper reason: two requests that deadlocked against each
/// other were, by construction, running at the same time, so an unjittered
/// backoff hands them the same next instant and they collide again. Spreading
/// them is the whole of what the wait is for.
///
/// Unlike `crate::queue`'s ladder, which mixes a job's own id, this draws from
/// a random source, because a transaction has nothing to mix: the two
/// conflicting requests share their statements and their timing and differ in
/// nothing this function can read. `rand` is already this crate's, for the
/// jitter above, so the draw costs no dependency and reopens no question under
/// `rule:packaging/a-c-dependency-answers-two-questions`.
pub(super) fn retry_backoff(taken: u32) -> std::time::Duration {
    // Clamped before the shift rather than after: sixteen rungs is already past
    // the cap for this base, and a shift by 32 is undefined rather than
    // saturating.
    let ceiling = RETRY_BACKOFF
        .saturating_mul(1_u32 << taken.min(16))
        .min(RETRY_BACKOFF_CAP);
    let nanos = u64::try_from(ceiling.as_nanos()).unwrap_or(u64::MAX);
    std::time::Duration::from_nanos(rand::rng().random_range(0..=nanos))
}

/// § 7's suspension between two attempts — the coroutine waits and the core
/// does not.
///
/// [`nvs_runtime::host::Host::sleep`] and deliberately not the bounded park
/// [`wait_for_slot`] takes, on that trait's own distinction between the two: a
/// backoff is a wait **for the clock**, with no state a peer could change to
/// end it early, and nothing that could wake it would mean anything — a
/// connection coming free says nothing about a deadlock that is already broken.
///
/// With no host on the thread the wait still happens, blocking, for
/// `Core\Time::sleep`'s reason: there is no scheduler under the call, so there
/// is no neighbour for
/// `rule:http-server/a-core-is-never-blocked-on-a-syscall`
/// 's tier-B failure to have as its victim.
///
/// # Errors
///
/// [`nvs_runtime::Ctx::cancel`]'s status for a task cancelled mid-wait, which
/// no `catch` sees. The attempt this waits after was already rolled back by the
/// conflict that ended it, so there is nothing left open to close.
pub(super) fn wait_between_attempts(ctx: &mut nvs_runtime::Ctx, taken: u32) -> Result<(), Fault> {
    let waited = retry_backoff(taken);
    match nvs_runtime::host::with_current(|host| host.sleep(waited)) {
        Some(nvs_runtime::host::Woken::Cancelled) => Err(ctx.cancel()),
        Some(nvs_runtime::host::Woken::Elapsed) => Ok(()),
        None => {
            std::thread::sleep(waited);
            Ok(())
        }
    }
}

/// § 7's whole transaction, from the `BEGIN` to the answer, over a connection
/// it is handed.
///
/// Hoisted out of [`nvs_core_db_connection_transaction`] so that the retry rule
/// — four conditions over two conflict channels, and the one part of § 7 that
/// belongs to no driver — has a caller other than a member that can only reach
/// a real server. [`Attempts`] is where what that buys is written down.
///
/// **The two channels differ only in the shape a conflict arrives in.** The
/// commit's refusal is an `io::Error` carrying its own [`nvs_db::ServerError`];
/// a conflict a statement *inside* the callable raised is a pending exception on
/// the context instead, so its kind is read back off that object's
/// [`nvs_runtime::KIND_SLOT`] and turned into a [`nvs_db::DbErrorKind`] by
/// [`error_kind_of`]. Both then ask [`nvs_db::DbErrorKind::is_retryable`],
/// which is the one place the rule lives, and both wait
/// [`wait_between_attempts`] before the re-run.
///
/// # Errors
///
/// The callable's own failure, re-raised exactly as it left it where no attempt
/// is left to spend; `Core\Db\RolledBack` where the callable abandoned the
/// scope; and [`statement_failure`]'s rendering of a refusal by the `BEGIN` or
/// by the command that closes the level.
pub(super) fn transacted(
    ctx: &mut nvs_runtime::Ctx,
    attempts: &mut impl Attempts,
    call: &Attempted,
) -> Result<Value, Fault> {
    let block = call.block;
    let mut left = call.retries;

    // § 11's readers, once for the whole call: every command below runs on
    // the one connection, and a retry does not change what is watching.
    // Read here for [`QueryWatch`]'s reason — the connection holds the
    // context for as long as each command does.
    let watch = QueryWatch::of(ctx, &block);

    loop {
        // § 7 retries **outermost transactions only**, and the depth before
        // the `BEGIN` is the only thing that says which this call is —
        // re-running a nested callable would re-run it inside an outer
        // transaction the conflict has already aborted.
        let open = attempts.depth(ctx)?;
        let outermost = open == 0;
        // The level this attempt's `BEGIN` opens, counted from 1 for the
        // outermost, which is how `crate::queue::level_closed` is told which
        // level ended.
        let level = open + 1;
        // § 11's event covers § 7's own commands as well as the statements
        // inside them: a trace that showed the callable's writes but not the
        // `BEGIN` and the `COMMIT` around them would put the transaction's
        // whole cost on its last statement. The driver answers with the
        // span because only it knows whether the depth made this a
        // `SAVEPOINT` — [`nvs_db::PgConn::begin`] and its MySQL twin own
        // that, and the second of them spends two round trips where an
        // isolation level was asked for.
        let opened = attempts
            .begin(ctx, call.isolation, call.read_only)?
            .map_err(|refused| statement_failure(TRANSACTION_MEMBER, &block, None, &refused))?;
        file_span(ctx, watch, &block, opened);

        // The block name is handed on rather than looked up again: a
        // transaction refuses under the same `[db.<name>]` its connection
        // does, and the slot is the only place that name lives.
        let scope = crate::instance::build(
            &TRANSACTION,
            [
                Value::uint(call.key),
                owned(block),
                Value::bool(true),
                Value::null(),
            ],
        );
        let outcome = nvs_runtime::call_callable(ctx, call.callable, &[scope]);

        // Closed before the outcome is acted on, so that a `$tx` the callable
        // stored somewhere is already refusing by the time this call returns
        // — and closed on every path, which is why it is not inside a
        // branch. An attempt that retries gets its own scope object below,
        // for the same reason it gets its own `BEGIN`.
        let receiver = crate::instance::receiver(scope, &TRANSACTION, "transaction")?;
        crate::instance::set_slot(receiver, SCOPE_AT, Value::bool(false));
        let held = crate::instance::slot(receiver, REASON_AT);
        let abandoned = held.as_text().map(str::to_owned);
        discard(scope);

        let answered = match outcome {
            Ok(value) => value,
            Err(fault) => {
                // The callable's own conflict, under the same four
                // conditions the commit's is. It is a `Fault::Pending`
                // here, so the kind is read off the still-pending object
                // rather than off an `io::Error` this path never has —
                // borrowing it, because a failure that turns out not to be
                // retryable is re-raised exactly as the callable left it.
                let conflicted = ctx
                    .pending_slot(ThrownClass::DbError.name(), nvs_runtime::KIND_SLOT)
                    .and_then(error_kind_of)
                    .is_some_and(nvs_db::DbErrorKind::is_retryable);
                let retry = outermost && left > 0 && abandoned.is_none() && conflicted;
                // Best effort as before, and filed on the path where it
                // worked: an undo the server ran is a statement the trace
                // owes an entry, and one it refused leaves no span to file.
                let undone = attempts.roll_back(ctx).ok().and_then(Result::ok);
                if let Some(span) = undone {
                    file_span(ctx, watch, &block, span);
                }
                // A job the callable enqueued at this level went with it.
                crate::queue::level_closed(ctx, call.key, level, false);
                if !retry {
                    return Err(fault);
                }
                // Cleared before the next attempt: the retry is this
                // frame's decision that the throw did not happen as far as
                // the caller is concerned, and a pending failure left on
                // the context would surface against whatever ran next.
                drop(ctx.take_thrown());
                let taken = u32::try_from(call.retries.saturating_sub(left)).unwrap_or(u32::MAX);
                left -= 1;
                // After the rollback and after the context is clear, so a
                // cancellation arriving mid-wait ends a call with nothing open
                // and nothing pending.
                wait_between_attempts(ctx, taken)?;
                continue;
            }
        };

        // The driver's own error rather than the `Fault` it renders to: the
        // retry rule branches on § 8's kind, which only [`nvs_db`] can put
        // there and only this shape still carries.
        let closed = if abandoned.is_some() {
            attempts.roll_back(ctx)
        } else {
            attempts.commit(ctx)
        };
        let closed = match closed {
            Ok(closed) => closed,
            Err(fault) => {
                discard(answered);
                return Err(fault);
            }
        };
        // A job enqueued at this level is durable from here, handed to the
        // level around this one, or gone, and `rule:concurrency/a-push-wakes-an-idle-worker`
        // announces it only in the first case. A commit the server refused
        // leaves nothing committed, so it is judged as a rollback.
        crate::queue::level_closed(ctx, call.key, level, abandoned.is_none() && closed.is_ok());

        // On two of the three paths the callable's answer is not this call's,
        // and this frame owns the only reference to it.
        let refused = match closed {
            Ok(span) => {
                // The command that closed the level, whichever it was, and
                // filed before this call returns rather than after — the
                // answer below leaves by three different paths.
                file_span(ctx, watch, &block, span);
                return match abandoned {
                    Some(reason) => {
                        discard(answered);
                        Err(Fault::thrown_as(ThrownClass::DbRolledBack, reason))
                    }
                    None => Ok(answered),
                };
            }
            Err(refused) => refused,
        };
        discard(answered);

        // § 7's `{retries: n}`, and the four conditions are all of it: an
        // outermost transaction, an attempt left, nothing that asked to be
        // rolled back, and a conflict the driver says may be re-run.
        let conflicted =
            nvs_db::ServerError::of(&refused).is_some_and(|server| server.kind.is_retryable());
        if outermost && left > 0 && abandoned.is_none() && conflicted {
            // The rung is how many re-runs this call has already spent, so the
            // first wait is one base and each one after it doubles. Read before
            // the decrement, where it is still 0.
            let taken = u32::try_from(call.retries.saturating_sub(left)).unwrap_or(u32::MAX);
            left -= 1;
            wait_between_attempts(ctx, taken)?;
            continue;
        }
        return Err(statement_failure(
            TRANSACTION_MEMBER,
            &block,
            None,
            &refused,
        ));
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Transaction::rollBack(string $reason): void` — `rule:core-classes/db-transactions`'s
    /// second hazard, closed by doing both things at once.
    ///
    /// **The flag is what the owning frame acts on and the throw is what the
    /// program sees**, and neither alone is enough. A throw on its own is
    /// catchable, so a `catch (Throwable)` between here and
    /// [`nvs_core_db_connection_transaction`] could leave the work committed;
    /// a flag on its own is `setRollbackOnly`, which every layer has to
    /// remember to check and one of them will not. Recording the reason in the
    /// receiver's own slot is what makes the decision survive the catch.
    ///
    /// **There is deliberately no way to roll back and carry on.** § 7 gives
    /// the member a `void` return because it always throws: a program that
    /// wants the writes it has made kept has not asked for a transaction, and
    /// one that wants to try again puts the retry outside the callable, where
    /// the second attempt gets its own `BEGIN`.
    fn nvs_core_db_transaction_roll_back(_ctx, args: [2]) {
        // Through the same guard every delegated member runs, and for the same
        // reason: a `$tx` that outlived its call has no scope to abandon.
        transaction_of(args[0], "rollBack")?;
        let receiver = crate::instance::receiver(args[0], &TRANSACTION, "rollBack")?;
        // Unreachable from source: parameter 0 is a `string` in [`TRANSACTION`]
        // above, so `E0401` refuses anything else before this runs.
        let reason = args[1]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{ROLL_BACK} expected a `string` reason, got tag {}",
                    args[1].tag_byte()
                ))
            })?
            .to_owned();
        crate::instance::set_slot(receiver, REASON_AT, owned(args[1]));
        Err(Fault::thrown_as(ThrownClass::DbRolledBack, reason))
    }
}

/// Drops a reference this frame owns, for a value it is not handing back.
///
/// The mirror of [`owned`], and it exists for one member: `transaction` builds
/// the scope object and receives the callable's answer, and on the paths where
/// the transaction did not commit neither of them reaches Novis code at all.
pub(super) fn discard(value: Value) {
    #[expect(
        unsafe_code,
        reason = "the reference released here is one this frame took — from \
                  `instance::build`, or from `call_callable`, which hands back a \
                  value the caller owns"
    )]
    unsafe {
        value.release();
    }
}

/// A value read out of an array or a slot, with a reference of the caller's
/// own.
///
/// Every read below borrows — [`NvsArray::get`], `value_at` and
/// [`crate::instance::slot`] all hand back a reference the holder still owns —
/// so this is the one place the second one is taken, rather than an `unsafe`
/// block at each of the members that hands a held value out.
pub(super) fn owned(value: Value) -> Value {
    #[expect(
        unsafe_code,
        reason = "the entry is owned by an array the receiver holds, which \
                  outlives this call, so the value handed back needs a \
                  reference of its own"
    )]
    unsafe {
        value.retain();
    }
    value
}

/// What [`begin_test_transaction`] and [`roll_back_test_transaction`] are
/// worded as when they read a `[db.<name>]` block or file a connection —
/// [`CONNECT`]'s slot, filled by the mechanism rather than by a member, since
/// no `Core` member is on the stack when either of these runs.
const TEST_TRANSACTION: &str = "`rule:testing/db-transaction`'s transaction";

/// A [`Fault`] as the one line a caller outside this crate reports it on.
///
/// The runner has no `catch` to hand a fault to and no context to record one
/// against — the connection is opened *around* a test rather than inside it —
/// so the two functions below answer a `String` and this is where a fault
/// becomes one. Every variant renders as its own message; a
/// [`Fault::Pending`] carries none, and cannot arise here anyway, no Novis
/// code running between the `BEGIN` and this call.
fn refusal(fault: &Fault) -> String {
    match fault {
        Fault::Thrown(_, message)
        | Fault::ThrownWithSlots(_, message, _)
        | Fault::Fatal(message) => message.to_string(),
        _ => "the connection could not be reached".to_owned(),
    }
}

/// `rule:testing/db-transaction`'s
/// outer transaction: `[db.<name>]` opened on `ctx` and left inside a `BEGIN`,
/// answering the key it is filed under.
///
/// **It is `ctx` that makes this § 17 rather than a second `Core\Db`.** The
/// connection is memoized on the context it is opened on
/// ([`open_named`]), so a `Core\Db::connect` the test *itself* makes under the
/// same name reaches this very connection — and therefore this open
/// transaction. That is the whole of § 17's second sentence: the test's own
/// `Core\Db::transaction` sees a non-zero [`nvs_db::PgConn::depth`] and opens a
/// `SAVEPOINT` instead of a `BEGIN`, with no special case anywhere for it. The
/// caller passes the *test's* context and not the suite's, which is why
/// `nvs-cli`'s runner arms this from inside the isolate's own program.
///
/// **What it spends:** one pooled connection for the length of one test, held
/// by that test's context and released to [`nvs_runtime::pool`] when it ends —
/// the same connection a test that called `connect` itself would have held, so
/// § 17 costs a `db:` test nothing beyond the two round trips of its `BEGIN`
/// and its `ROLLBACK`.
///
/// # Errors
///
/// The message to report against the test: no `[db.<name>]` block, a block that
/// cannot be opened, or a `BEGIN` the server refused.
pub fn begin_test_transaction(ctx: &mut nvs_runtime::Ctx, name: &str) -> Result<u64, String> {
    let key =
        open_named(ctx, name, true, None, TEST_TRANSACTION).map_err(|fault| refusal(&fault))?;
    transacting(ctx, key, TEST_TRANSACTION)
        .map_err(|fault| refusal(&fault))?
        .begin(None, false)
        .map_err(|error| format!("`[db.{name}]` refused the transaction: {error}"))?;
    Ok(key)
}

/// § 17's other half: every level open on the connection filed under `key`,
/// rolled back, so the test's writes are gone and the connection is poolable.
///
/// **A loop and not one `ROLLBACK`, because the depth is not this function's to
/// assume.** § 7's callable closes its own level on every path out of it, so the
/// depth is back to the one [`begin_test_transaction`] opened for every test
/// that returned or threw — but a test that ended by cancellation or by a
/// contained panic is a test whose callable did not return, and a connection
/// left one level in is one [`nvs_runtime::pool`] closes rather than reuses.
/// The loop terminates because a `ROLLBACK` that the driver accepted is what
/// lowers the depth (`nvs_db::pg`'s `roll_back`), and a refused one returns
/// here.
///
/// # Errors
///
/// The message to report against the test: the key names no connection, or the
/// server refused a `ROLLBACK`.
pub fn roll_back_test_transaction(ctx: &mut nvs_runtime::Ctx, key: u64) -> Result<(), String> {
    loop {
        let mut open = transacting(ctx, key, TEST_TRANSACTION).map_err(|fault| refusal(&fault))?;
        if open.depth() == 0 {
            return Ok(());
        }
        open.roll_back()
            .map_err(|error| format!("the transaction refused its rollback: {error}"))?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::{Ctx, OutputSink, call};

    /// `rule:core-classes/db-transactions`'s two halves, and the second is the one a forwarding body
    /// would pass while still drifting.
    ///
    /// **A transaction runs a callable**: [`TRANSACTION_ROW`] takes one
    /// `callable` and answers at *its* `T`, which is what lets a transaction
    /// wrap an existing expression without retyping it. Its consequence is
    /// asserted as an absence — § 7 removes `commit`, `rollBack` and
    /// `inTransaction` from the connection outright, and a pair a program can
    /// leave half-done is exactly what nesting made unnecessary.
    ///
    /// **`Transaction implements Queryable by $connection`** is asserted over
    /// [`CONNECTION`]'s whole roster rather than member by member, so a member
    /// added there fails here until [`TRANSACTION`] carries it: the rows must
    /// be *identical*, symbol included, since `rule:classes/no-traits`'s delegation is one
    /// body reached through either handle and a second body would agree on
    /// name and arity on the day it was written and on nothing afterwards.
    /// `stream` and `streamAs` are owed on both and so are outside the sweep
    /// by construction; [`BEYOND_QUERYABLE`] is the one set that is outside it
    /// on purpose, and the sweep checks that list from both ends first.
    #[test]
    fn a_transaction_is_a_callable_and_transaction_is_a_queryable() {
        assert_eq!(TRANSACTION_ROW.name, "transaction");
        // § 7's `$fn`, and R9's allowance that the callable may declare no
        // parameter at all is why the arity is not sayable in the row.
        assert_eq!(TRANSACTION_ROW.names, ["fn"]);
        assert_eq!(
            format!("{:?}", TRANSACTION_ROW.params),
            format!(
                "{:?}",
                [
                    CoreTy::CallableSig(&[CoreTy::Instance(TRANSACTION_NAME)], &CoreTy::Var("T")),
                    CoreTy::Options(TRANSACTION_OPTIONS)
                ]
            ),
            "§ 7's `transaction` takes the callable and R2's one trailing bag"
        );
        // § 7's three options, in its order, at its defaults — asserted as the
        // whole bag rather than one lookup each, so an option added without
        // being specified fails here too. The defaults are the half a call site
        // never writes and so the half nothing else would catch: `isolation`
        // absent leaves the server's own level standing, and `retries` at 0 is
        // § 7's argument that a side-effecting callable is not re-run unasked.
        assert_eq!(
            TRANSACTION_OPTIONS
                .iter()
                .map(|option| (option.name, format!("{:?}", option.default)))
                .collect::<Vec<_>>(),
            [
                ("isolation", format!("{:?}", Const::Null)),
                ("readOnly", format!("{:?}", Const::Bool(false))),
                ("retries", format!("{:?}", Const::Uint(0))),
            ]
        );
        assert_eq!(
            format!("{:?}", TRANSACTION_ROW.return_ty),
            format!("{:?}", CoreTy::Var("T")),
            "§ 7's `: T` — the member's answer is the callable's own"
        );

        // "no `commit()`, no `rollBack()` on the connection and no
        // `inTransaction()`": nesting removed the reason each existed, so
        // their absence is the decision rather than an unlanded row.
        for member in CONNECTION.members() {
            assert!(
                !matches!(
                    member.name,
                    "begin" | "commit" | "rollBack" | "savepoint" | "inTransaction"
                ),
                "`{}` declares `{}` — § 7 replaced that surface with a callable",
                CONNECTION.name,
                member.name
            );
        }

        // § 18's second table — the rows `Connection` has *beyond* `Queryable`
        // — is the only thing outside the sweep, and it is asserted from both
        // ends here so that naming a row there cannot quietly excuse one from
        // the comparison below. [`BEYOND_QUERYABLE`] owns why the exception is
        // a list rather than a weaker check.
        for name in BEYOND_QUERYABLE {
            assert!(
                CONNECTION.instance.iter().any(|row| row.name == *name),
                "`{name}` is named as a connection-only row and is not a row at all"
            );
            assert!(
                !TRANSACTION.instance.iter().any(|row| row.name == *name),
                "`{name}` is named as a connection-only row and `{}` carries it",
                TRANSACTION.name
            );
        }

        let declared: Vec<String> = CONNECTION
            .instance
            .iter()
            .filter(|row| !BEYOND_QUERYABLE.contains(&row.name))
            .map(|row| format!("{row:?}"))
            .collect();
        let forwarded: Vec<String> = TRANSACTION
            .instance
            .iter()
            .filter(|row| CONNECTION.instance.iter().any(|had| had.name == row.name))
            .map(|row| format!("{row:?}"))
            .collect();
        assert_eq!(
            declared, forwarded,
            "`{}` is `{}`'s query surface delegated, not restated: every row \
             agrees down to its symbol and its position",
            TRANSACTION.name, CONNECTION.name
        );

        // What the delegation is allowed to add, in full: § 7's own hazard,
        // and it is on the transaction because that is what holds the flag.
        let own: Vec<&str> = TRANSACTION
            .instance
            .iter()
            .map(|row| row.name)
            .filter(|name| !CONNECTION.instance.iter().any(|had| had.name == *name))
            .collect();
        assert_eq!(own, ["rollBack"]);

        // Load-bearing rather than tidy, per [`TRANSACTION`]'s own doc: one
        // statement path reads either handle, which is what makes the shared
        // symbols above reachable at all.
        assert_eq!(
            &TRANSACTION.slots[..CONNECTION.slots.len()],
            CONNECTION.slots
        );
    }

    /// `rule:core-classes/db-transactions`'s second hazard, asserted at the seam where Doctrine's
    /// `setRollbackOnly()` loses: the decision is on the receiver, so throwing
    /// it away does not undo it.
    ///
    /// **Dropping the [`Fault`] is what an intervening `catch (Throwable)`
    /// does**, and this case does exactly that between the raise and the read
    /// — [`nvs_core_db_transaction_roll_back`] records the reason in
    /// [`REASON_SLOT`] *and* throws, and
    /// [`nvs_core_db_connection_transaction`] reads that slot rather than the
    /// exception it did not catch. A member that only threw would pass every
    /// assertion here but the last one.
    ///
    /// **The read happens after the scope is closed**, which is the order the
    /// owning frame runs in: the `$tx` is refusing further work by then, and
    /// the decision it holds still has to be legible. Both sides are named,
    /// since a transaction nobody abandoned reads back as the same `null` a
    /// broken recording would.
    #[test]
    fn roll_back_survives_an_intervening_catch_of_throwable() {
        let scope = crate::instance::build(
            &TRANSACTION,
            [
                Value::uint(1),
                Value::str(NvsStr::new(b"main")),
                Value::bool(true),
                Value::null(),
            ],
        );
        let receiver =
            crate::instance::receiver(scope, &TRANSACTION, "rollBack").expect("a built instance");
        assert!(
            crate::instance::slot(receiver, REASON_AT)
                .as_text()
                .is_none(),
            "a transaction nobody abandoned carries no reason, which is the \
             `commit` half of the same read"
        );

        let mut ctx = Ctx::new(OutputSink::Sink);
        let reason = Value::str(NvsStr::new(b"the cart is gone"));
        if call(
            nvs_core_db_transaction_roll_back,
            &mut ctx,
            &[scope, reason],
        )
        .is_ok()
        {
            panic!("§ 7 gives `rollBack` a `void` return because it always throws");
        }
        assert_eq!(
            ctx.pending_class().as_deref(),
            Some(r"Core\Db\RolledBack"),
            "§ 7's signal is the class it names, not a bare `RuntimeError`"
        );
        assert_eq!(ctx.pending().as_deref(), Some("the cart is gone"));

        // The catch, and this is the whole of one: clearing the pending
        // exception is what a `catch (Throwable)` frame does. Everything below
        // runs in the state `setRollbackOnly()` cannot recover from.
        assert!(ctx.take_pending().is_some());
        assert!(ctx.pending().is_none());

        crate::instance::set_slot(receiver, SCOPE_AT, Value::bool(false));
        assert_eq!(
            crate::instance::slot(receiver, REASON_AT).as_text(),
            Some("the cart is gone"),
            "the flag outlives both the exception and the scope — it is what \
             the owning frame rolls back on"
        );

        // And the scope guard is the first hazard, still closed: a `$tx` the
        // callable stored cannot abandon a transaction that has moved on.
        assert!(matches!(
            transaction_of(scope, "rollBack"),
            Err(Fault::Thrown(ThrownClass::Logic, _))
        ));

        discard(reason);
        discard(scope);
    }

    /// [`Attempts`] with no server behind it: every command succeeds, and the
    /// case reads back the order they were asked in.
    ///
    /// Nothing here is scripted to *fail*, because the conflict § 7 retries on
    /// arrives from the callable rather than from the connection — the half no
    /// driver could induce, and the whole reason [`transacted`] takes its
    /// connection as an argument. The depth is 0 so every attempt is
    /// outermost, which is one of the four conditions the retry rule reads.
    struct Scripted {
        /// Every command [`transacted`] asked for, in order.
        asked: Vec<&'static str>,
    }

    impl Scripted {
        /// One command, recorded and answered with a span a trace could file.
        fn ran(
            &mut self,
            command: &'static str,
        ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
            self.asked.push(command);
            Ok(Ok(nvs_db::QuerySpan::opened(
                nvs_db::Driver::Postgres,
                command,
            )))
        }
    }

    impl Attempts for Scripted {
        fn depth(&mut self, _ctx: &mut Ctx) -> Result<u32, Fault> {
            Ok(0)
        }

        fn begin(
            &mut self,
            _ctx: &mut Ctx,
            _isolation: Option<nvs_db::Isolation>,
            _read_only: bool,
        ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
            self.ran("BEGIN")
        }

        fn commit(&mut self, _ctx: &mut Ctx) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
            self.ran("COMMIT")
        }

        fn roll_back(
            &mut self,
            _ctx: &mut Ctx,
        ) -> Result<std::io::Result<nvs_db::QuerySpan>, Fault> {
            self.ran("ROLLBACK")
        }
    }

    thread_local! {
        /// How many times [`conflicts_once`] has been entered on this thread.
        ///
        /// A thread local rather than a field on [`Scripted`]: the callback is
        /// reached through an `extern "C"` address and has no `self` to read.
        static ATTEMPTS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }

    /// § 7's callable as a deadlock makes it behave: the first attempt throws a
    /// retryable `Core\Db\DbError`, and every one after it returns.
    ///
    /// **The throw is built by [`statement_failure`] off a real
    /// [`nvs_db::ServerError`]** rather than assembled here, so what the loop
    /// reads back through [`nvs_runtime::Ctx::pending_slot`] is the object a
    /// PostgreSQL `40P01` actually produces —
    /// `a_kind_written_into_a_throw_reads_back_as_itself` holds the other half
    /// of that round trip.
    #[expect(
        unsafe_code,
        reason = "`call_callable` passes exactly these two live values, each \
                  retained for this callee to release, and `run_helper` \
                  discharges the rest of the helper ABI's pointer contract"
    )]
    unsafe extern "C" fn conflicts_once(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        unsafe {
            nvs_runtime::run_helper(ctx, args, 2, out, |_ctx, args| {
                // The exit sweep a compiled callee owes: the receiver and the
                // one declared parameter, both retained on the way in.
                for slot in args {
                    discard(*slot);
                }
                if ATTEMPTS.with(|entered| entered.replace(entered.get() + 1)) > 0 {
                    return Ok(Value::int(2));
                }
                Err(deadlocked())
            })
        }
    }

    /// What a statement on `[db.main]` throws when PostgreSQL reports a deadlock.
    fn deadlocked() -> Fault {
        let block = Value::str(NvsStr::new(b"main"));
        let deadlocked = statement_failure(
            TRANSACTION_MEMBER,
            &block,
            None,
            &std::io::Error::other(nvs_db::ServerError {
                kind: nvs_db::DbErrorKind::Deadlock,
                sql_state: String::from("40P01"),
                severity: String::from("ERROR"),
                message: String::from("deadlock detected"),
                constraint: None,
                driver_code: None,
                backend: "postgresql",
            }),
        );
        discard(block);
        deadlocked
    }

    /// Spec § 10's root shape and § 8's `Core\Db\DbError` under it, arriving the one way a
    /// context takes a table — the playbook's `Ctx::class_desc` bullet.
    ///
    /// § 8's class has to be *resolvable* for a throw of it to be read back:
    /// `pending_slot` answers `None` for a context with no exception class
    /// installed.
    fn db_error_class() -> nvs_runtime::ErrorClass {
        const THROWABLE: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut classes = nvs_runtime::ClassTable::new();
        let root = classes.define("RuntimeError", &THROWABLE, &[]);
        classes.define(
            ThrownClass::DbError.name(),
            &[
                "message",
                "previous",
                "backtrace",
                "location",
                "kind",
                "sqlState",
                "driverCode",
                "constraint",
                "sql",
            ],
            &[root],
        );
        nvs_runtime::ErrorClass::new(std::sync::Arc::new(classes), root)
    }

    /// A `callable` whose `invoke` is `invoke` and which declares one
    /// parameter — the `$tx` § 7 hands its callable.
    ///
    /// `nvs_runtime::call_callable` reads exactly three things off a callable
    /// value, so this is a whole one: its class's callable bit, the arity in
    /// its own slot, and the address in the class's `CALLABLE_INVOKE` row —
    /// see `nvs_runtime::ClassDesc::is_callable`. The table is leaked
    /// because a descriptor's *address* is its identity and it must outlive
    /// every instance made from it, which is `crate::instance`'s own rule; the
    /// test process exiting is what reclaims it.
    fn callable_of(invoke: nvs_runtime::NvsFn) -> Value {
        let mut table = nvs_runtime::ClassTable::new();
        let id = table.define("{callable}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![nvs_runtime::MethodRow {
                name: nvs_runtime::CALLABLE_INVOKE.to_owned(),
                code: invoke as *const u8,
                // Read off the object's own slots below rather than off this
                // row — see `nvs_runtime::MethodRow`.
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        table.set_callable(id);
        let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives \
                      every instance made from it — `NvsObj::new`'s whole \
                      obligation"
        )]
        let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
        object.set_field(nvs_runtime::CALLABLE_ARITY_SLOT, Value::int(1));
        object.set_field(
            nvs_runtime::CALLABLE_PARAM_TAGS_SLOT,
            Value::int(i64::from(nvs_runtime::CALLABLE_PARAM_TAG_ANY)),
        );
        Value::object(object)
    }

    /// `rule:core-classes/db-transactions`'s `{retries: n}`, at the seam that belongs to no driver: a
    /// deadlock inside the callable re-runs it, and the second attempt's answer
    /// is the call's.
    ///
    /// **The connection is handed in rather than filed**, which is what
    /// [`Attempts`] exists for. [`transacting`] downcasts to a real
    /// `nvs_db::PgConn` and no `-p nvs-stdlib` test can build one, so until
    /// [`transacted`] took its connection as an argument the retry rule — the
    /// one half of § 7 a driver is never asked to implement — had no caller a
    /// test could reach.
    ///
    /// **The conflict is induced the way a server induces one**: the callable
    /// throws what [`statement_failure`] renders a `40P01` into, so the loop's
    /// decision goes through [`nvs_runtime::Ctx::pending_slot`] and § 8's
    /// normalised kind exactly as it does in a request.
    ///
    /// **Counting the attempts is not enough, so the commands are read back in
    /// order.** A loop that re-ran the callable but left the aborted attempt
    /// open, or that opened no second `BEGIN`, would pass a count alone. The
    /// pending failure is asserted *gone* for the same reason: a retry the
    /// caller is never told about must leave nothing for the next member to
    /// trip over.
    ///
    /// **Both sides of the bound**, since a loop that always retried would
    /// pass the first half — § 7's default is 0, and at 0 the same conflict
    /// reaches the caller with the callable run once.
    // covers: Core\Db\Connection::transaction
    #[test]
    fn retries_recover_an_induced_deadlock() {
        let block = Value::str(NvsStr::new(b"main"));
        let callable = callable_of(conflicts_once);
        let attempted = |retries| Attempted {
            key: 1,
            block,
            callable,
            isolation: None,
            read_only: false,
            retries,
        };

        // § 8's class has to be *resolvable* or the retry cannot happen at
        // all: a loop reading the kind off the throw would see no conflict
        // and re-raise.
        ATTEMPTS.with(|entered| entered.set(0));
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(db_error_class());
        let mut recovered = Scripted { asked: Vec::new() };
        let answered = transacted(&mut ctx, &mut recovered, &attempted(1))
            .expect("§ 7 re-runs a deadlocked callable, and the second attempt commits");
        assert_eq!(
            answered.as_int(),
            Some(2),
            "the call answers the *retried* attempt's return, not the conflict"
        );
        assert_eq!(
            ATTEMPTS.with(std::cell::Cell::get),
            2,
            "one attempt per `BEGIN`, and § 7 spends the one retry it was given"
        );
        assert_eq!(
            recovered.asked,
            ["BEGIN", "ROLLBACK", "BEGIN", "COMMIT"],
            "each attempt gets its own `BEGIN`, and the aborted one is undone \
             before the next opens"
        );
        assert!(
            ctx.pending().is_none(),
            "the retry is this frame's decision that the throw did not happen, \
             so nothing may be left pending for the next member"
        );

        ATTEMPTS.with(|entered| entered.set(0));
        let mut refused = Scripted { asked: Vec::new() };
        let raised = transacted(&mut ctx, &mut refused, &attempted(0))
            .expect_err("§ 7's default is 0, and a conflict with no attempt left is the caller's");
        assert!(
            matches!(raised, Fault::Pending(_)),
            "the callable's own failure travels on unchanged: {raised:?}"
        );
        assert_eq!(
            ctx.pending_class().as_deref(),
            Some(r"Core\Db\DbError"),
            "§ 8's class, still pending exactly as the callable left it"
        );
        assert_eq!(ATTEMPTS.with(std::cell::Cell::get), 1);
        assert_eq!(
            refused.asked,
            ["BEGIN", "ROLLBACK"],
            "the attempt is still rolled back — what 0 removes is the re-run, \
             not the undo"
        );

        drop(ctx.take_thrown());
        discard(callable);
        discard(block);
    }

    /// What [`enqueues`] does when § 7 calls it.
    #[derive(Clone, Copy)]
    struct Enqueue {
        /// The connection the job is announced on.
        key: u64,
        /// How many transactions to open inside this one before the enqueue.
        nested: u32,
        /// Whether a job is announced at all.
        announces: bool,
        /// Whether the callable throws after it, which rolls its transaction back.
        throws: bool,
    }

    thread_local! {
        /// What [`enqueues`] does on this thread. A thread local for [`ATTEMPTS`]'s reason.
        static ENQUEUE: std::cell::Cell<Enqueue> = const {
            std::cell::Cell::new(Enqueue { key: 0, nested: 0, announces: false, throws: false })
        };

        /// The process bell's count as [`enqueues`] last read it, inside the outermost
        /// transaction and after everything it ran there.
        static RINGS_INSIDE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    /// § 7's callable as a request that enqueues a job writes it: what `Core\Queue::push` does
    /// once its insert has run, which is [`crate::queue::announce`], inside as many nested
    /// transactions as [`ENQUEUE`] names.
    #[expect(
        unsafe_code,
        reason = "`call_callable` passes exactly these two live values, each \
                  retained for this callee to release, and `run_helper` \
                  discharges the rest of the helper ABI's pointer contract"
    )]
    unsafe extern "C" fn enqueues(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        unsafe {
            nvs_runtime::run_helper(ctx, args, 2, out, |ctx, args| {
                for slot in args {
                    discard(*slot);
                }
                let plan = ENQUEUE.with(std::cell::Cell::get);
                if plan.nested > 0 {
                    ENQUEUE.with(|held| {
                        held.set(Enqueue {
                            nested: plan.nested - 1,
                            ..plan
                        });
                    });
                    let answered = transacted_on(ctx, plan.key);
                    RINGS_INSIDE.with(|read| read.set(crate::queue::Bell::process().rings()));
                    return answered;
                }
                if plan.announces {
                    crate::queue::announce(ctx, plan.key)?;
                }
                RINGS_INSIDE.with(|read| read.set(crate::queue::Bell::process().rings()));
                if plan.throws {
                    return Err(deadlocked());
                }
                Ok(Value::int(1))
            })
        }
    }

    /// One `transaction` on the connection filed under `key`, running [`enqueues`].
    fn transacted_on(ctx: &mut Ctx, key: u64) -> Result<Value, Fault> {
        let block = Value::str(NvsStr::new(b"main"));
        let callable = callable_of(enqueues);
        let answered = transacted(
            ctx,
            &mut Filed { key },
            &Attempted {
                key,
                block,
                callable,
                isolation: None,
                read_only: false,
                retries: 0,
            },
        );
        discard(callable);
        discard(block);
        answered
    }

    /// `rule:concurrency/a-push-wakes-an-idle-worker`'s order: the bell rings once the job is
    /// committed, and a job that was rolled back rings nothing.
    ///
    /// **A real connection and § 7's own loop**, because what decides the ring is the depth the
    /// connection reports and the level `transacted` closes. SQLite is the backend that needs no
    /// server. The enqueue is [`crate::queue::announce`], the call `Core\Queue::push` makes once
    /// its insert has run, so the statement itself is not what is under test here.
    ///
    /// **Counted, and counted alone.** Each step reads how far [`crate::queue::Bell::process`]'s
    /// count moved, inside the transaction and after it, under the lock that keeps another case
    /// from ringing it meanwhile.
    // covers: Core\Queue::push
    #[test]
    fn a_push_rings_after_the_commit_and_a_rollback_rings_nothing() {
        let _alone = crate::queue::bell::tests::process_bell_alone();
        let bell = crate::queue::Bell::process();

        let block = nvs_config::tree::Database {
            driver: Some(String::from("sqlite")),
            path: Some(String::from(":memory:")),
            ..nvs_config::tree::Database::default()
        };
        let target = nvs_db::SqliteTarget::resolve(&block).expect("a `sqlite` block resolves");
        let conn = nvs_db::sqlite::open(&target).expect("an in-memory database opens");
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(db_error_class());
        let key = ctx.hold_open_connection(None, None, Box::new(nvs_db::Connection::Sqlite(conn)));

        // How far the count moved by the end of the outermost callable, and by the end of the call.
        let moved = |ctx: &mut Ctx, plan: Enqueue| {
            let before = bell.rings();
            ENQUEUE.with(|held| held.set(plan));
            let answered = transacted_on(ctx, plan.key);
            let inside = RINGS_INSIDE.with(std::cell::Cell::get) - before;
            (answered.is_ok(), inside, bell.rings() - before)
        };
        let plan = Enqueue {
            key,
            nested: 0,
            announces: true,
            throws: false,
        };

        let before = bell.rings();
        crate::queue::announce(&mut ctx, key).expect("the connection is filed");
        assert_eq!(
            bell.rings() - before,
            1,
            "a push outside a transaction is its own committed statement, and it did not ring"
        );

        assert_eq!(
            moved(&mut ctx, plan),
            (true, 0, 1),
            "a push inside a transaction rings once, and only after that transaction commits"
        );

        assert_eq!(
            moved(
                &mut ctx,
                Enqueue {
                    throws: true,
                    ..plan
                }
            ),
            (false, 0, 0),
            "a push whose transaction rolled back rang for a job that does not exist"
        );
        drop(ctx.take_thrown());
        assert_eq!(
            moved(
                &mut ctx,
                Enqueue {
                    announces: false,
                    ..plan
                }
            ),
            (true, 0, 0),
            "the commit after a rollback rang for the job that rollback undid"
        );

        assert_eq!(
            moved(&mut ctx, Enqueue { nested: 2, ..plan }),
            (true, 0, 1),
            "a push inside nested transactions rings once, at the outermost commit"
        );

        assert_eq!(
            moved(
                &mut ctx,
                Enqueue {
                    nested: 1,
                    throws: true,
                    ..plan
                }
            ),
            (false, 0, 0),
            "a push a nested transaction rolled back rang anyway"
        );
        drop(ctx.take_thrown());
    }

    /// `rule:core-classes/db-transactions`'s backoff, asserted as bounds over the whole ladder rather
    /// than as numbers: the draw is random, so there is no value to name.
    ///
    /// **Both halves are asserted because either alone passes for the wrong
    /// reason.** A `retry_backoff` answering its ceiling every time grows and
    /// caps correctly and loses the entire point — two requests that
    /// deadlocked against each other are handed one next instant and collide
    /// again — while one drawing over a fixed range is jittered and retries a
    /// hopeless conflict at the same rate forever.
    #[test]
    fn the_backoff_is_exponential_jittered_and_capped() {
        for taken in 0..24_u32 {
            let ceiling = RETRY_BACKOFF
                .saturating_mul(1_u32 << taken.min(16))
                .min(RETRY_BACKOFF_CAP);
            for _ in 0..64 {
                let drawn = retry_backoff(taken);
                assert!(
                    drawn <= ceiling,
                    "rung {taken} drew past the ceiling full jitter is uniform under"
                );
                assert!(
                    drawn <= RETRY_BACKOFF_CAP,
                    "rung {taken} drew past the cap a `uint` of retries is bounded by"
                );
            }
        }

        let drawn: std::collections::BTreeSet<_> = (0..64).map(|_| retry_backoff(4)).collect();
        assert!(drawn.len() > 1, "an unjittered backoff draws one value");

        // The ceiling doubles per rung, which no single draw shows, so it is
        // asserted on the maximum of enough of them to reach past the rung
        // below: 256 draws all landing in rung 0's tenth of rung 3's range is
        // not a flake anyone will see.
        let low = (0..256).map(|_| retry_backoff(0)).max().expect("256 draws");
        let high = (0..256).map(|_| retry_backoff(3)).max().expect("256 draws");
        assert!(high > low, "rung 3's range did not reach past rung 0's");
    }
}
