//! `Core\Budget` — the three numbers a request may ask about its own memory,
//! and nothing about the process it is running in.
//!
//! `rule:observability/memory-is-three-numbers-on-core-budget` is the whole
//! surface: `memoryHeld()` is what this request holds right now, `memoryPeak()`
//! the high-water mark of that figure, and `memoryLimit()` the ceiling both are
//! measured against. Together they answer in one expression the question
//! the rule was written for — *how close to the
//! ceiling did this request come* — which neither of PHP's two functions could
//! answer without `ini_get('memory_limit')` and a suffix parser beside it.
//!
//! # Why the peak is a runtime counter and not a reading taken here
//!
//! Because by the time a program asks, the spike is gone.
//! `rule:observability/a-memory-peak-is-recorded-not-asked-for` is the rule and
//! [`nvs_runtime::budget`] is where it lives: Novis releases memory when the
//! last reference dies, so a request that decoded a 90 MB payload and returned
//! a 2 KB summary reads 2 KB from `memoryHeld` and is indistinguishable, at
//! that moment, from one that never grew. So the mark moves in the allocator,
//! and every member here is a field read off [`nvs_runtime::Ctx`].
//!
//! # This class is the request, and `Core\Os` is the process
//!
//! `rule:observability/memory-is-three-numbers-on-core-budget`'s split, and the reason this class
//! exists at all rather than a fourth member landing on [`crate::os`]: PHP's
//! `memory_get_usage` was answering two questions under one name, and a
//! per-request figure sitting among host facts is read as process memory by
//! everyone who has not been told otherwise. `Core\Os::residentBytes` is the
//! process's resident set and only ever grows; these three are this request's
//! and end with it.
//!
//! **Held, not used**, for that rule's reason: the breach a request gets is
//! rendered *"N bytes held against a ceiling of M"*, and a member disagreeing
//! with the error text about the same quantity is a second vocabulary.
//!
//! # `uint`, where the record writes `int`
//!
//! All three are byte counts that cannot be negative, and
//! [`nvs_runtime::Ctx::memory_used`] and [`nvs_runtime::Ctx::memory_peak`]
//! already floor at zero rather than reporting a request that released what it
//! inherited as owing bytes. `Core\Os::residentBytes` is spelled the same way
//! for the same reason, which matters more than the record's PHP-flavoured
//! signature block: two byte counts a program is expected to compare should not
//! be two integer types.

use nvs_runtime::Value;

use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Budget";

/// `Core\Budget`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Reports how much memory the current request uses, the most it has used so far, and \
            its memory limit. The numbers are for this request only, not for the whole server \
            process.",
};

/// `Core\Budget`'s registry rows — the three numbers, and the module docs above
/// own why there is no fourth and no `$real_usage` boolean beside any of them.
///
/// The other class that needs no request in hand, and the last of this
/// run of them: `Core\Budget`, which answers what *this* request holds
/// rather than what the process does. `rule:observability/memory-is-three-numbers-on-core-budget` is the roster —
/// three readings, no `$real_usage` boolean — and [`crate::budget`]'s module
/// doc owns why the peak is recorded by the allocator rather than sampled
/// here, and why the process's own memory stays on `Core\Os`.
// rank: 690
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "memoryHeld",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_budget_memory_held",
            doc: Some(&MEMORY_HELD_DOC),
        },
        CoreMethod {
            name: "memoryPeak",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_budget_memory_peak",
            doc: Some(&MEMORY_PEAK_DOC),
        },
        CoreMethod {
            name: "memoryLimit",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_budget_memory_limit",
            doc: Some(&MEMORY_LIMIT_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Budget::memoryHeld`'s reference card — `rule:core-api/reference-card`.
const MEMORY_HELD_DOC: MethodDoc = MethodDoc {
    short: "Returns the number of bytes **this request** uses right now. Novis frees a value as \
            soon as nothing uses it. So the number goes down again when a request is done with a \
            large value.",
    params: &[],
    ret: "The number of bytes this request uses. It does not count the rest of the server \
          process. `Core\\Os::residentBytes` returns the memory of the whole process.",
    errors: &[],
};

/// `Core\Budget::memoryPeak`'s reference card — `rule:core-api/reference-card`.
const MEMORY_PEAK_DOC: MethodDoc = MethodDoc {
    short: "Returns the highest value `memoryHeld()` has had during this request. The number is \
            recorded at every allocation, so it includes memory that was already freed again.",
    params: &[],
    ret: "The highest number of bytes this request has used. It is never below what \
          `memoryHeld()` returns. A program cannot reset it.",
    errors: &[],
};

/// `Core\Budget::memoryLimit`'s reference card — `rule:core-api/reference-card`.
const MEMORY_LIMIT_DOC: MethodDoc = MethodDoc {
    short: "Returns the memory limit that `memoryHeld()` and `memoryPeak()` are measured against. \
            It is the `[limits] memory` setting in bytes, minus the memory reserved for the limit \
            handler. You do not need to parse `Core\\Config::get('limits.memory')` yourself.",
    params: &[],
    ret: "The limit in bytes. It is `0` when the request has no memory limit.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_budget_memory_held" => (nvs_core_budget_memory_held as *const ()).cast(),
        "nvs_core_budget_memory_peak" => (nvs_core_budget_memory_peak as *const ()).cast(),
        "nvs_core_budget_memory_limit" => (nvs_core_budget_memory_limit as *const ()).cast(),
        _ => return None,
    })
}

/// A byte count as the `uint` all three members answer with, saturating rather
/// than wrapping on a host where `usize` is wider than `u64`.
fn bytes(count: usize) -> Value {
    Value::uint(u64::try_from(count).unwrap_or(u64::MAX))
}

nvs_runtime::nvs_helper! {
    /// `Core\Budget::memoryHeld(): uint` — replacing `memory_get_usage`.
    ///
    /// This request's share of the thread's live balance, which is what
    /// `Ctx::memory_used` already computes for the ceiling check; reading it
    /// here rather than the counter directly is what makes the figure a
    /// request's and not a worker thread's.
    fn nvs_core_budget_memory_held(ctx, _args: [0]) {
        Ok(bytes(ctx.memory_used()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Budget::memoryPeak(): uint` — replacing `memory_get_peak_usage`.
    ///
    /// The mark the allocator recorded, against the same zero point the held
    /// figure uses. The module doc owns why it is recorded rather than derived
    /// from a reading taken here.
    fn nvs_core_budget_memory_peak(ctx, _args: [0]) {
        Ok(bytes(ctx.memory_peak()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Budget::memoryLimit(): uint` — the ceiling the other two are read
    /// against, which PHP reaches only as `ini_get('memory_limit')`.
    ///
    /// `Ctx::memory_limit` is the number `Ctx::over_memory_limit` compares
    /// against, so this is the ceiling in the sense a breach means, and its
    /// zero already means *no cap*.
    fn nvs_core_budget_memory_limit(ctx, _args: [0]) {
        Ok(bytes(ctx.memory_limit()))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, OutputSink};

    use super::{
        CLASS, nvs_core_budget_memory_held, nvs_core_budget_memory_limit,
        nvs_core_budget_memory_peak,
    };
    use crate::registry::CoreTy;

    /// What a member answered, as the `uint` every one of them returns.
    fn answer(member: nvs_runtime::NvsFn, ctx: &mut Ctx) -> u64 {
        nvs_runtime::call(member, ctx, &[])
            .expect("a budget reading answers")
            .as_uint()
            .expect("a budget reading is a uint")
    }

    /// The peak keeps a spike the held figure has already given back, and the
    /// ceiling is the one both are read against.
    ///
    /// The three are asserted together because the pair of readings is what the
    /// class is for: a member answering plausibly on its own line still fails
    /// here if `memoryPeak` is `memoryHeld` under another name, which is the
    /// shape the rule refuses.
    // covers: Core\Budget::memoryHeld, Core\Budget::memoryPeak, Core\Budget::memoryLimit
    #[test]
    fn the_peak_outlives_the_spike_the_held_figure_gives_back() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_memory_limit(64 << 20);
        let held = vec![0_u8; 1 << 20];
        let peak = answer(nvs_core_budget_memory_peak, &mut ctx);
        assert!(
            peak >= 1 << 20,
            "the mark missed a megabyte this request holds"
        );
        drop(held);
        assert!(
            answer(nvs_core_budget_memory_held, &mut ctx) < peak,
            "the release did not lower the held figure, so this asserts nothing"
        );
        assert_eq!(
            answer(nvs_core_budget_memory_peak, &mut ctx),
            peak,
            "the mark fell back with the held figure"
        );
        assert_eq!(
            answer(nvs_core_budget_memory_limit, &mut ctx),
            64 << 20,
            "the ceiling is not the one the request was armed with"
        );
    }

    /// An uncapped request answers `0` from the ceiling rather than a sentinel
    /// of its own.
    ///
    /// The rule names this reading explicitly, and it is the one value a caller
    /// has to branch on: `peak * 10 > limit * 9` is false for every request
    /// under no cap, which is the intended answer.
    // covers: Core\Budget::memoryLimit
    #[test]
    fn an_uncapped_request_reads_a_ceiling_of_zero() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        assert_eq!(
            answer(nvs_core_budget_memory_limit, &mut ctx),
            0,
            "a request under no cap invented a ceiling"
        );
    }

    /// Three members, all of them byte counts, and none of them a mode.
    ///
    /// The roster is the rule: a `$real_usage`-style boolean is what
    /// `rule:core-api/no-mode-strings` refuses, and the failure it guards
    /// against is a later member arriving with a parameter at all — every one
    /// of these takes none, because there is no second accounting to select.
    #[test]
    fn the_class_is_three_readings_and_no_mode_parameter() {
        let names: Vec<&str> = CLASS.methods.iter().map(|method| method.name).collect();
        assert_eq!(
            names,
            vec!["memoryHeld", "memoryPeak", "memoryLimit"],
            "`Core\\Budget` is three numbers and nothing else"
        );
        for method in CLASS.methods {
            assert!(
                method.params.is_empty() && method.names.is_empty(),
                "`Core\\Budget::{}` takes an argument, which is a second accounting to choose",
                method.name
            );
            assert!(
                matches!(method.return_ty, CoreTy::Uint),
                "`Core\\Budget::{}` answers something other than a byte count",
                method.name
            );
        }
        assert!(
            CLASS.instance.is_empty() && CLASS.slots.is_empty() && CLASS.constants.is_empty(),
            "`Core\\Budget` grew something to hold a budget on"
        );
    }
}
