//! Guards the *premise* of `rule:errors/propagation`.
//!
//! Novis propagates errors by checked return because `cranelift-jit` registers
//! no unwind tables for the code it emits, so the platform unwinder has nothing
//! to walk a JIT frame with. That is a property of a dependency, and it could
//! change. This test is how we find out.
//!
//! # How it works
//!
//! It compiles a small JIT function with the flags most likely to produce unwind
//! information (`is_pic=false`, `unwind_info=true`), finalizes it, and asks the
//! platform's own lookup whether a PC inside it has an unwind entry:
//! `RtlLookupFunctionEntry` on Windows, `_Unwind_Find_FDE` on Unix. Both consult
//! the tables a JIT would register at run time (`RtlAddFunctionTable`,
//! `__register_frame`) as well as the loaded images, so a registration shows up
//! here whichever way it is done. The test fails when an entry is found.
//!
//! The question is put to the platform directly. Provoking a panic beneath the
//! frame and seeing whether `catch_unwind` above it fires makes the result depend
//! on the stack layout of the test binary: on Windows x64 a PC with no function
//! entry is treated as a leaf, and whatever stale slot sits at `[RSP]` decides
//! where the walk goes next.
//!
//! A control asks the same lookup about a function of this binary, which does
//! have an entry, so a lookup that finds nothing for everything cannot pass the
//! test by accident. The control is skipped on macOS, where Rust code is
//! described by compact unwind records and `_Unwind_Find_FDE` reports DWARF
//! entries only.

use std::ffi::c_void;

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

/// The callee the JIT function calls, so that function has a frame of its own
/// rather than being a leaf an unwinder could step over without an entry.
extern "C" fn helper_that_doubles(x: i64) -> i64 {
    x * 2
}

#[test]
fn a_jit_frame_has_no_unwind_entry() {
    let mut flags = settings::builder();
    flags.set("use_colocated_libcalls", "false").unwrap();
    flags.set("is_pic", "false").unwrap();
    // Ask for unwind information explicitly, so this is a fair test of the
    // premise rather than a self-fulfilling one.
    flags.set("unwind_info", "true").unwrap();

    let isa = cranelift_native::builder()
        .expect("host is not a supported cranelift target")
        .finish(settings::Flags::new(flags))
        .expect("failed to construct the host ISA");

    let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    builder.symbol("helper_that_doubles", helper_that_doubles as *const u8);
    let mut module = JITModule::new(builder);

    let mut sig = module.make_signature();
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));

    let helper = module
        .declare_function("helper_that_doubles", Linkage::Import, &sig)
        .unwrap();
    let wrapper = module
        .declare_function("jit_wrapper", Linkage::Export, &sig)
        .unwrap();

    let mut ctx = module.make_context();
    let mut fn_ctx = FunctionBuilderContext::new();
    ctx.func.signature = sig;
    {
        let target_config = module.target_config();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fn_ctx);
        let helper_ref = module.declare_func_in_func(helper, b.func);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        b.seal_block(entry);
        let x = b.block_params(entry)[0];
        let call = b.ins().call(helper_ref, &[x]);
        let ret = b.inst_results(call)[0];
        b.ins().return_(&[ret]);
        b.finalize(target_config);
    }
    module.define_function(wrapper, &mut ctx).unwrap();
    module.clear_context(&mut ctx);
    module.finalize_definitions().unwrap();

    let code = module.get_finalized_function(wrapper);

    #[allow(
        unsafe_code,
        reason = "the function was just compiled with exactly this signature"
    )]
    // SAFETY: `jit_wrapper` was declared as `(i64) -> i64` above and the module
    // has been finalized, so the code pointer is valid and executable.
    let jit: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(code) };
    assert_eq!(
        jit(5),
        10,
        "the JIT function must run before its entry is looked up"
    );

    // One byte past the start is inside the prologue, which is the range an
    // unwind entry for this function would have to cover.
    let jit_pc = code.wrapping_add(1);

    if cfg!(not(target_os = "macos")) {
        let control = a_jit_frame_has_no_unwind_entry as *const u8;
        assert!(
            has_unwind_entry(control.wrapping_add(1)),
            "harness problem, not a result: the platform lookup found no unwind entry for a \
             function of this test binary either, so it cannot tell whether the JIT frame has one"
        );
    }

    assert!(
        !has_unwind_entry(jit_pc),
        "\n\
         ============================================================\n\
         The platform unwinder has an unwind entry for a Cranelift JIT\n\
         frame, which means cranelift-jit now registers unwind tables.\n\
         \n\
         rule:errors/propagation says what to do: rewrite its sentence\n\
         about unwind tables, and turn this canary around so it records\n\
         that the entry exists and fails if that changes back. Checked\n\
         returns stay, for the reasons the rule gives.\n\
         ============================================================"
    );
}

/// Whether the platform unwinder has an entry covering `pc`.
#[cfg(windows)]
fn has_unwind_entry(pc: *const u8) -> bool {
    #[allow(
        unsafe_code,
        reason = "declares the documented kernel32 export the unwinder itself uses"
    )]
    unsafe extern "system" {
        fn RtlLookupFunctionEntry(
            control_pc: u64,
            image_base: *mut u64,
            history_table: *mut c_void,
        ) -> *const c_void;
    }

    let mut image_base = 0_u64;
    #[allow(
        unsafe_code,
        reason = "the lookup only reads the loader's and the dynamic function tables"
    )]
    // SAFETY: `RtlLookupFunctionEntry` accepts any PC, writes only through
    // `image_base`, which is a valid local, and takes a null history table.
    let entry =
        unsafe { RtlLookupFunctionEntry(pc as u64, &raw mut image_base, std::ptr::null_mut()) };
    !entry.is_null()
}

/// The bases `_Unwind_Find_FDE` fills in beside the entry it returns.
#[cfg(unix)]
#[repr(C)]
struct DwarfEhBases {
    tbase: *const c_void,
    dbase: *const c_void,
    func: *const c_void,
}

/// Whether the platform unwinder has an entry covering `pc`.
#[cfg(unix)]
fn has_unwind_entry(pc: *const u8) -> bool {
    #[allow(
        unsafe_code,
        reason = "declares the unwinder export that libgcc and libunwind both provide"
    )]
    unsafe extern "C" {
        fn _Unwind_Find_FDE(pc: *const c_void, bases: *mut DwarfEhBases) -> *const c_void;
    }

    let mut bases = DwarfEhBases {
        tbase: std::ptr::null(),
        dbase: std::ptr::null(),
        func: std::ptr::null(),
    };
    #[allow(
        unsafe_code,
        reason = "the lookup only reads the loaded images' and the registered frame tables"
    )]
    // SAFETY: `_Unwind_Find_FDE` accepts any PC and writes only through
    // `bases`, which is a valid local of the layout it expects.
    let entry = unsafe { _Unwind_Find_FDE(pc.cast(), &raw mut bases) };
    !entry.is_null()
}
