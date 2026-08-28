//! The probe [ADR 0026](../../../docs/adr/0026-performance-measurement-methodology.md)
//! validates callgrind against: confirms `valgrind --tool=callgrind` produces
//! a bit-for-bit stable instruction count for code Cranelift JIT-compiled at
//! runtime, which is the premise the ADR's historical-dashboard leg rests on.
//!
//! Kept as the minimal reusable harness for that leg, not yet the leg itself
//! — there is no ndjson writer or dedicated-runner wiring here. Growing this
//! into the real per-commit workload roster is deferred per the ADR's
//! *Revisiting* section.
//!
//! ```text
//! cargo build --release -p nvs-abi-probe --example callgrind_spike
//! valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out \
//!     ./target/release/examples/callgrind_spike
//! callgrind_annotate /tmp/cg.out | head -20
//! ```

use std::hint::black_box;

use nvs_abi_probe::{Ctx, Helper, Value, call};

fn main() {
    let mut probe = nvs_abi_probe::Probe::new();
    let chain = probe.compile_chain(8, Helper::Double);
    let mut ctx = Ctx::new();

    for _ in 0..10_000u64 {
        black_box(call(chain, &mut ctx, black_box(Value::int(3))));
    }
}
