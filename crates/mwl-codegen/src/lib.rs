//! MWL's baseline Cranelift backend: [`mwl_ir`]'s CFG/SSA form in, native
//! code behind [ADR 0002](../../../docs/adr/0002-error-propagation.md)'s
//! calling convention out.
//!
//! This crate is the one place that knows *both* [`mwl_ir`] and
//! [`mwl_runtime`]. `mwl-runtime` deliberately depends on neither, so mapping
//! an [`mwl_ir::ir::Helper`] tag onto a symbol name from
//! [`mwl_runtime::symbols`] is this crate's job by design — see that crate's
//! own module docs.
//!
//! # The shape it emits
//!
//! Every compiled function has the one signature ADR 0002 makes normative:
//!
//! ```text
//! extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32
//! ```
//!
//! Nothing unwinds. Every call — a runtime helper, the safepoint slow path —
//! is followed by a compare-and-branch on the returned status, and a non-[`OK`]
//! status is returned onward unchanged. That pair of instructions is what
//! replaces a landing pad; `benches/abi-probe`'s `compile_chain` has measured
//! its cost since M0.
//!
//! ## Values are native, not tagged, wherever the type is known
//!
//! [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) settles every
//! operand type before lowering, so an `int` local lives in an `i64` register
//! and a `string` in a bare `StrHeader` pointer. A 16-byte
//! [`mwl_runtime::Value`] is *materialized* only where the ABI demands one —
//! at a call boundary and at the `out` slot — exactly as that type's own docs
//! say. [`ty::clif_ty`] is the whole of the mapping.
//!
//! ## The two flag checks
//!
//! Both are a load of one word from [`mwl_runtime::Ctx`] plus a
//! predicted-not-taken branch, and both are emitted unconditionally:
//!
//! * the **safepoint poll** at every [`mwl_ir::ir::InstKind::Safepoint`] —
//!   function entry and loop back edges, the project-start decision's two
//!   fixed sites;
//! * [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
//!   § 1's **debug-flags check** at every
//!   [`mwl_ir::ir::InstKind::StmtMarker`], branching to
//!   [`mwl_runtime::mwl_probe_stmt`].
//!
//! Neither is behind a flag or a build configuration: ADR 0018's whole
//! argument is that a request already running must be able to have coverage
//! turned on, which a compiled-in-advance instrumented tier cannot do. Their
//! all-bits-off cost is guarded in `benches/abi-probe`
//! (`tests/perf_guards.rs`), not asserted here.
//!
//! # Scope of this slice
//!
//! Narrow by authorization, not by accident — `.claude/loop-goal.md` allows
//! the first backend to be exactly as wide as `mwl run examples/hello.mwl`
//! requires, and each gap below is a missing *lowering*, not a missing
//! decision:
//!
//! 1. **No MWL-level call.** [`mwl_ir::ir::InstKind::Call`]/`New`/`FieldGet`/
//!    `FieldSet`/`Concat`/`ArrayNew`/`ArrayGet`/`ArraySet`/`ArrayAppend` all
//!    report [`CodegenError::Unsupported`] naming the instruction. `Call`
//!    needs a symbol table over the unit's own functions plus an object/array
//!    representation for its operands, neither of which exists; `Concat`
//!    additionally needs a `mwl_str_concat` runtime primitive `mwl-runtime`
//!    has not grown yet.
//! 2. **ADR 0018's call-site probe is not emitted**, because there is no
//!    compiled call site to attach it to. Its `TRACE`/`PROFILE` entry/exit
//!    pair lands in [`emit::Emitter::emit_call`]'s single path together with
//!    item 1, which is exactly where that ADR says it belongs. The
//!    statement-boundary probe, which does have sites today, is emitted.
//! 3. **The error path leaks.** A non-`OK` status returns immediately without
//!    releasing the refcounted locals still live in the frame.
//!    [`mwl_ir`] itself does not model an error edge yet (see
//!    `InstKind::HelperCall`'s own doc comment), so there is nothing to lower
//!    a cleanup path *from*; both halves land together.
//! 4. **A string literal still allocates on every evaluation.** The literal's
//!    bytes are emitted into the unit's data section, but
//!    `InstKind::ConstStr` calls `mwl_str_new` over them rather than pointing
//!    at a pinned-refcount immortal header — `mwl-runtime`'s known gap 3,
//!    which named codegen as the missing half.
//! 5. **[`mwl_ir::Ty::Mixed`] cannot be materialized.** Writing one into a
//!    `Value` needs the runtime type tag nothing has decided yet (`mwl-ir`'s
//!    known gap 5); a `mixed`-typed *return of nothing* still works, since
//!    that writes `null`.
//! 6. **Executable memory is never freed.** [`Unit`] holds its `JITModule` for
//!    the process's lifetime; `cranelift_jit::JITModule::free_memory` is
//!    `unsafe` and needs the "no compiled frame is still live" proof that
//!    [ADR 0017](../../../docs/adr/0017-hot-reload-without-restart.md)'s
//!    pointer-swap reclamation is the real home for. A one-shot `mwl run`
//!    exits before it matters.

mod emit;
mod ty;

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, ModuleError};
use mwl_ir::Program;
use mwl_runtime::MwlFn;
use rustc_hash::FxHashMap;

pub use ty::clif_ty;

/// Why a program could not be compiled.
///
/// Every variant is an *engine* failure — a shape this backend does not lower
/// yet, or a host that cannot host a JIT. None of them is a user diagnostic:
/// `mwl run` has already run `mwl_types::check_program` and reported every
/// diagnostic before a single instruction is emitted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CodegenError {
    /// An IR shape this slice does not lower — see the crate docs' scope list.
    #[error("mwl-codegen does not lower {0} yet")]
    Unsupported(String),
    /// Cranelift rejected the generated code, or the JIT module did.
    #[error("internal error: cranelift rejected the code generated for `{function}`: {source}")]
    Cranelift {
        /// The MWL function being compiled.
        function: String,
        /// What Cranelift reported.
        source: Box<ModuleError>,
    },
    /// The host machine cannot run a Cranelift JIT at all.
    #[error("this host is not a supported cranelift target: {0}")]
    UnsupportedHost(String),
}

/// One compiled compilation unit: the native code, plus the module that owns
/// the pages it lives on.
///
/// Holding the [`JITModule`] is what keeps the code mapped — see the crate
/// docs' known gap 6 on why nothing unmaps it.
pub struct Unit {
    /// Kept alive for its pages; never read again after `compile` returns.
    _module: JITModule,
    entries: FxHashMap<String, *const u8>,
}

impl std::fmt::Debug for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut names: Vec<&str> = self.entries.keys().map(String::as_str).collect();
        names.sort_unstable();
        f.debug_struct("Unit").field("functions", &names).finish()
    }
}

impl Unit {
    /// The compiled function `name` names, ready to call — through
    /// [`mwl_runtime::call`], which is the safe wrapper over [`MwlFn`]'s
    /// pointer contract.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "handing back a JIT-compiled code pointer as a callable is a \
                  transmute with no safe spelling; the function was declared \
                  with exactly `MwlFn`'s signature in `compile` and \
                  `finalize_definitions` has made its pages executable"
    )]
    pub fn function(&self, name: &str) -> Option<MwlFn> {
        let code = *self.entries.get(name)?;
        Some(unsafe { std::mem::transmute::<*const u8, MwlFn>(code) })
    }
}

/// Compiles every function in `program` to native code.
///
/// # Errors
///
/// [`CodegenError::Unsupported`] for an IR shape this slice does not lower
/// (crate docs, scope list), [`CodegenError::Cranelift`] if Cranelift rejects
/// what was generated — always a bug in this crate, never in the input — and
/// [`CodegenError::UnsupportedHost`] on a machine Cranelift has no backend
/// for.
pub fn compile(program: &Program) -> Result<Unit, CodegenError> {
    let mut jit = Jit::new(None)?;
    for (index, function) in program.functions.iter().enumerate() {
        jit.compile_function(index, function)?;
    }
    jit.finish()
}

/// Compiles every function in `program` and returns the generated machine
/// code as text, one section per function, *instead* of a callable [`Unit`].
///
/// This is what `mwl run --dump-asm` prints. It compiles through exactly the
/// same path [`compile`] does — same ISA flags, same emitted probes — with
/// Cranelift's disassembler switched on, so what it prints is the code that
/// would have run rather than a second, differently-configured rendering.
/// Nothing is executed.
///
/// # Errors
///
/// The same three cases [`compile`] reports, for the same reasons.
pub fn disassemble(program: &Program) -> Result<String, CodegenError> {
    let mut jit = Jit::new(Some(String::new()))?;
    for (index, function) in program.functions.iter().enumerate() {
        jit.compile_function(index, function)?;
    }
    // `finish` still has to run: `finalize_definitions` is what resolves the
    // relocations, and a unit that cannot be linked is not a unit whose
    // disassembly should be reported as if it were fine.
    let disasm = jit.disasm.take().unwrap_or_default();
    jit.finish()?;
    Ok(disasm)
}

/// The JIT module under construction, plus the imported runtime symbols every
/// compiled function shares.
struct Jit {
    module: JITModule,
    ctx: codegen::Context,
    fn_ctx: FunctionBuilderContext,
    sigs: Signatures,
    /// One entry per emitted `ConstStr`, so data-object names stay unique.
    literals: usize,
    entries: Vec<(String, cranelift_module::FuncId)>,
    /// Set only by [`disassemble`]: the accumulated text of every function's
    /// generated code. `None` is the ordinary compile, which asks Cranelift
    /// for no disassembly at all and so pays nothing for this field.
    disasm: Option<String>,
}

/// The four signatures the runtime exports, beyond the helper ABI itself.
///
/// `mwl-runtime`'s entry points are deliberately not all the same shape:
/// `mwl_str_new`/`mwl_str_retain`/`mwl_str_release` operate on a raw
/// `StrHeader` pointer with no context and no `Value`, because they are memory
/// primitives rather than language operations, and `mwl_probe_stmt` returns
/// nothing because a coverage probe cannot fail. Each therefore gets its own
/// signature here rather than being forced through [`Signatures::helper`].
struct Signatures {
    /// ADR 0002's calling convention — `(ctx, args, out) -> status`.
    helper: Signature,
    /// `mwl_safepoint(ctx) -> status`.
    safepoint: Signature,
    /// `mwl_probe_stmt(ctx, stmt_id)`.
    probe: Signature,
    /// `mwl_str_new(ptr, len) -> *mut StrHeader`.
    str_new: Signature,
    /// `mwl_str_retain(ptr)` / `mwl_str_release(ptr)`.
    refcount: Signature,
}

impl Jit {
    fn new(disasm: Option<String>) -> Result<Self, CodegenError> {
        let mut flags = settings::builder();
        for (name, value) in [
            // A JIT resolves every call through an absolute address, and the
            // pages are its own — the same configuration benches/abi-probe has
            // measured MWL's costs under since M0.
            ("use_colocated_libcalls", "false"),
            ("is_pic", "false"),
            ("opt_level", "speed"),
        ] {
            flags
                .set(name, value)
                .map_err(|e| CodegenError::UnsupportedHost(e.to_string()))?;
        }

        let isa = cranelift_native::builder()
            .map_err(|e| CodegenError::UnsupportedHost(e.to_owned()))?
            .finish(settings::Flags::new(flags))
            .map_err(|e| CodegenError::UnsupportedHost(e.to_string()))?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        for (name, address) in mwl_runtime::symbols() {
            builder.symbol(name, address);
        }

        let module = JITModule::new(builder);
        let sigs = Signatures::new(&module);
        Ok(Self {
            ctx: module.make_context(),
            fn_ctx: FunctionBuilderContext::new(),
            module,
            sigs,
            literals: 0,
            entries: Vec::new(),
            disasm,
        })
    }

    /// Compiles one function. `index` only disambiguates the Cranelift symbol
    /// name: an MWL function name is not a valid symbol (`<script>` is the
    /// first counter-example), and two classes may declare the same method
    /// name.
    fn compile_function(
        &mut self,
        index: usize,
        function: &mwl_ir::Function,
    ) -> Result<(), CodegenError> {
        let symbol = format!("mwl{index}_{}", sanitize(&function.name));
        let id = self
            .module
            .declare_function(&symbol, Linkage::Local, &self.sigs.helper)
            .map_err(|source| CodegenError::Cranelift {
                function: function.name.clone(),
                source: Box::new(source),
            })?;

        self.ctx.func.signature = self.sigs.helper.clone();
        // Must be set per function: `Module::clear_context` resets it along
        // with the rest of the context.
        self.ctx.set_disasm(self.disasm.is_some());
        let result = emit::emit_function(
            &mut self.module,
            &mut self.ctx,
            &mut self.fn_ctx,
            &self.sigs,
            &mut self.literals,
            function,
        );
        if let Err(error) = result {
            self.module.clear_context(&mut self.ctx);
            return Err(error);
        }

        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|source| CodegenError::Cranelift {
                function: function.name.clone(),
                source: Box::new(source),
            })?;
        self.collect_disasm(&function.name);
        self.module.clear_context(&mut self.ctx);
        self.entries.push((function.name.clone(), id));
        Ok(())
    }

    /// Appends the just-compiled function's disassembly, if one was asked for.
    ///
    /// Called between `define_function` and `clear_context`, which is the only
    /// window the compiled code is still attached to the context.
    fn collect_disasm(&mut self, name: &str) {
        let Some(buffer) = self.disasm.as_mut() else {
            return;
        };
        let vcode = self
            .ctx
            .compiled_code()
            .and_then(|code| code.vcode.as_deref());
        buffer.push_str("; ");
        buffer.push_str(name);
        buffer.push('\n');
        match vcode {
            // Cranelift has no disassembler for every backend it can emit
            // for; saying so beats printing an empty section.
            None => buffer.push_str(";   <no disassembly available on this target>\n"),
            Some(text) => {
                buffer.push_str(text);
                if !text.ends_with('\n') {
                    buffer.push('\n');
                }
            }
        }
        buffer.push('\n');
    }

    fn finish(mut self) -> Result<Unit, CodegenError> {
        self.module
            .finalize_definitions()
            .map_err(|source| CodegenError::Cranelift {
                function: "<unit>".to_owned(),
                source: Box::new(source),
            })?;
        let entries = self
            .entries
            .iter()
            .map(|(name, id)| (name.clone(), self.module.get_finalized_function(*id)))
            .collect();
        Ok(Unit {
            _module: self.module,
            entries,
        })
    }
}

impl Signatures {
    fn new(module: &JITModule) -> Self {
        let ptr = types::I64;

        let mut helper = module.make_signature();
        helper.params.push(AbiParam::new(ptr)); // ctx
        helper.params.push(AbiParam::new(ptr)); // args
        helper.params.push(AbiParam::new(ptr)); // out
        helper.returns.push(AbiParam::new(types::I32)); // status

        let mut safepoint = module.make_signature();
        safepoint.params.push(AbiParam::new(ptr));
        safepoint.returns.push(AbiParam::new(types::I32));

        let mut probe = module.make_signature();
        probe.params.push(AbiParam::new(ptr));
        probe.params.push(AbiParam::new(types::I32));

        let mut str_new = module.make_signature();
        str_new.params.push(AbiParam::new(ptr));
        str_new.params.push(AbiParam::new(ptr));
        str_new.returns.push(AbiParam::new(ptr));

        let mut refcount = module.make_signature();
        refcount.params.push(AbiParam::new(ptr));

        Self {
            helper,
            safepoint,
            probe,
            str_new,
            refcount,
        }
    }
}

/// Reduces an MWL function name to something a linker symbol may contain.
///
/// Not a mangling scheme: [`Jit::compile_function`]'s index already supplies
/// uniqueness, so this only has to keep the name readable in a disassembly.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}
