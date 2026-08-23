//! Emits one [`mwl_ir::Function`] as Cranelift IR.
//!
//! The whole file is a walk of already-checked, already-lowered input: by the
//! time an [`mwl_ir::ir::Function`] reaches here, `mwl_types::check_program`
//! has proven it well-typed and `mwl_ir::lower` has settled every operand's
//! representation. Nothing is re-checked; a shape this slice cannot lower is a
//! [`CodegenError::Unsupported`] naming it, never a diagnostic.
//!
//! # Blocks and phis
//!
//! [`mwl_ir::ir::InstKind::Phi`] becomes a Cranelift block parameter. The IR
//! always emits a block's phis as a prefix of its instruction list — every
//! producer in `mwl_ir::lower` writes them into a freshly created merge or
//! loop-header block before anything else — so the translation is positional:
//! the *n*th leading phi is the *n*th block parameter, and every jump into
//! that block supplies its own incoming value for each. A phi appearing after
//! a non-phi instruction would break that correspondence, so it is rejected
//! rather than silently mistranslated.
//!
//! # The status check
//!
//! Every call this file emits — a runtime helper, `mwl_safepoint` — is
//! followed by the compare-and-branch [ADR 0002](../../../docs/adr/0002-error-propagation.md)
//! puts in place of a landing pad, and a non-`OK` status returns onward
//! unchanged. `mwl_probe_stmt` and the refcount primitives are the exceptions,
//! and only because they return no status at all: neither can fail.

use cranelift::prelude::*;
use cranelift_jit::JITModule;
use cranelift_module::{DataDescription, FuncId, Linkage, Module};
use mwl_ir::Ty;
use mwl_ir::ids::{BlockId, ValueId};
use mwl_ir::ir::{
    BasicBlock, BinOp, Function, Helper, Inst, InstKind, Terminator, ThrowableOp, UnOp,
};
use mwl_runtime::{DEBUG_FLAGS_OFFSET, OK, SAFEPOINT_OFFSET, THROWN, Tag, Value as MwlValue};
use rustc_hash::FxHashMap;

use crate::ty::{clif_ty, tag_of};
use crate::{Classes, CodegenError, Signatures};

/// Size of one 16-byte [`mwl_runtime::Value`], as an offset multiplier.
const VALUE_SIZE: i32 = 16;

/// `align_shift` for a 16-byte-aligned stack slot: 2^4 == 16.
const VALUE_ALIGN_SHIFT: u8 = 4;

/// Memory flags for a load or store this frame fully controls — a stack slot
/// it just allocated, or the `out` pointer its caller promised is writable.
fn trusted() -> MemFlagsData {
    MemFlagsData::trusted()
}

/// Memory flags for reading [`mwl_runtime::Ctx`]'s two hot words.
///
/// Deliberately *not* [`MemFlagsData::trusted`]: `trusted` asserts nothing about
/// aliasing today, but the safepoint word is written from outside the running
/// frame (a CPU-limit watchdog, a cancellation), so the load must stay a real
/// load rather than becoming something Cranelift may prove redundant. Marking
/// it `readonly` — which is what a future "this is immutable" annotation would
/// mean — would be exactly wrong.
fn ctx_word() -> MemFlagsData {
    MemFlagsData::new().with_notrap()
}

/// Everything about the compilation *unit* that emitting one function needs:
/// the runtime signatures, the unit's own function and class tables, and the
/// running literal counter that keeps data-object names unique.
///
/// Bundled rather than passed one by one because every field has the same
/// lifetime and the same "read this, don't rebuild it" role — and because
/// eight parameters is where the shape stops being readable.
pub(crate) struct UnitTables<'a> {
    pub sigs: &'a Signatures,
    /// Every function the unit defines, by MWL name — see
    /// [`crate::Jit::compile_all`] for why it is complete before any body is
    /// emitted.
    pub functions: &'a FxHashMap<String, FuncId>,
    /// Every class the unit declares — see [`crate::Classes`].
    pub classes: &'a Classes,
    /// One entry per emitted `ConstStr`, so data-object names stay unique.
    pub literals: &'a mut usize,
}

/// Emits `f` into `ctx.func`, which the caller has already given the ABI
/// signature.
pub(crate) fn emit_function(
    module: &mut JITModule,
    ctx: &mut codegen::Context,
    fn_ctx: &mut FunctionBuilderContext,
    tables: UnitTables<'_>,
    f: &Function,
) -> Result<(), CodegenError> {
    let UnitTables {
        sigs,
        functions,
        classes,
        literals,
    } = tables;
    let target_config = module.target_config();
    let mut b = FunctionBuilder::new(&mut ctx.func, fn_ctx);

    // A dedicated Cranelift entry block carrying the ABI parameters, jumping
    // straight into the IR's own entry block. Keeping the two separate means
    // the IR entry block is translated exactly like every other block, with no
    // special case for "this one also has the function's parameters".
    let abi_entry = b.create_block();
    b.append_block_params_for_function_params(abi_entry);
    b.switch_to_block(abi_entry);
    let ctx_p = b.block_params(abi_entry)[0];
    let args_p = b.block_params(abi_entry)[1];
    let out_p = b.block_params(abi_entry)[2];

    let mut blocks = FxHashMap::default();
    let mut phi_counts = FxHashMap::default();
    for block in &f.blocks {
        let clif = b.create_block();
        let phis = leading_phis(block)?;
        for inst in &block.insts[..phis] {
            let ty = inst
                .ty
                .ok_or_else(|| internal("a phi with no representation type"))?;
            let clif_ty = clif_ty(ty).ok_or_else(|| internal("a phi of representation `void`"))?;
            b.append_block_param(clif, clif_ty);
        }
        // A landing block is entered carrying the status that sent control
        // there — from a call site's own compare-and-branch, or as the
        // constant `THROWN` from a `Terminator::Throw`. It is a block
        // parameter rather than a re-read of anything, because the two entry
        // paths have nothing in common to re-read.
        if is_landing(block) {
            if phis > 0 {
                return Err(internal("a landing block with phis"));
            }
            b.append_block_param(clif, types::I32);
        }
        blocks.insert(block.id.index(), clif);
        phi_counts.insert(block.id.index(), phis);
    }

    let ir_entry = *blocks
        .get(&f.entry.index())
        .ok_or_else(|| internal("the entry block is not among the function's blocks"))?;
    b.ins().jump(ir_entry, &[]);

    let mut emitter = Emitter {
        b,
        module,
        sigs,
        functions,
        classes,
        literals,
        f,
        values: FxHashMap::default(),
        blocks,
        phi_counts,
        frefs: FxHashMap::default(),
        callee_refs: FxHashMap::default(),
        ctx_p,
        args_p,
        out_p,
        landing_status: None,
    };
    emitter.emit_blocks()?;
    emitter.b.seal_all_blocks();
    emitter.b.finalize(target_config);
    Ok(())
}

/// How many instructions at the head of `block` are phis, rejecting a phi that
/// appears anywhere else — see this module's docs.
fn leading_phis(block: &BasicBlock) -> Result<usize, CodegenError> {
    let count = block
        .insts
        .iter()
        .take_while(|i| matches!(i.kind, InstKind::Phi { .. }))
        .count();
    if block.insts[count..]
        .iter()
        .any(|i| matches!(i.kind, InstKind::Phi { .. }))
    {
        return Err(internal(
            "a phi that is not part of its block's leading run",
        ));
    }
    Ok(count)
}

/// Whether this block is one of `mwl_ir`'s landing blocks — the ones
/// [`mwl_ir::ir::Inst::on_error`] and [`Terminator::Throw`] branch to.
///
/// Read off the terminator rather than carried as a flag: only a landing
/// block ends in [`Terminator::Propagate`] or [`Terminator::Catch`], so the
/// terminator already is the marker.
fn is_landing(block: &BasicBlock) -> bool {
    matches!(
        block.term,
        Terminator::Propagate { .. } | Terminator::Catch { .. }
    )
}

fn internal(what: &str) -> CodegenError {
    CodegenError::Unsupported(format!("{what} (this is a bug in mwl-ir or mwl-codegen)"))
}

struct Emitter<'a, 'f> {
    b: FunctionBuilder<'f>,
    module: &'a mut JITModule,
    sigs: &'a Signatures,
    /// Every function the unit defines, by MWL name — see
    /// [`crate::Jit::compile_all`] for why it is complete before any body is
    /// emitted.
    functions: &'a FxHashMap<String, FuncId>,
    /// Every class the unit declares — see [`crate::Classes`].
    classes: &'a Classes,
    literals: &'a mut usize,
    f: &'a Function,
    /// Every SSA value defined so far, with the representation it was defined
    /// at — the IR carries that on the defining instruction, and an operand
    /// needs it back to pick an instruction (`sdiv` vs `udiv` vs `fdiv`).
    values: FxHashMap<u32, (Value, Ty)>,
    blocks: FxHashMap<u32, Block>,
    phi_counts: FxHashMap<u32, usize>,
    frefs: FxHashMap<&'static str, codegen::ir::FuncRef>,
    /// The same cache as `frefs`, for the unit's *own* functions.
    callee_refs: FxHashMap<String, codegen::ir::FuncRef>,
    ctx_p: Value,
    args_p: Value,
    out_p: Value,
    /// The status parameter of the landing block currently being emitted, or
    /// `None` for an ordinary block — see [`is_landing`].
    landing_status: Option<Value>,
}

impl Emitter<'_, '_> {
    fn emit_blocks(&mut self) -> Result<(), CodegenError> {
        for block in &self.f.blocks {
            let start = self.block(block.id)?;
            self.b.switch_to_block(start);

            let phis = *self
                .phi_counts
                .get(&block.id.index())
                .ok_or_else(|| internal("a block with no recorded phi count"))?;
            for (index, inst) in block.insts[..phis].iter().enumerate() {
                let result = inst
                    .result
                    .ok_or_else(|| internal("a phi defining no value"))?;
                let ty = inst
                    .ty
                    .ok_or_else(|| internal("a phi with no representation type"))?;
                let param = self.b.block_params(start)[index];
                self.values.insert(result.index(), (param, ty));
            }

            self.landing_status =
                if is_landing(block) {
                    Some(*self.b.block_params(start).get(phis).ok_or_else(|| {
                        internal("a landing block with no status parameter appended")
                    })?)
                } else {
                    None
                };

            let mut cur = start;
            for inst in &block.insts[phis..] {
                cur = self.emit_inst(cur, inst)?;
            }
            self.emit_terminator(cur, block)?;
        }
        Ok(())
    }

    fn block(&self, id: BlockId) -> Result<Block, CodegenError> {
        self.blocks
            .get(&id.index())
            .copied()
            .ok_or_else(|| internal("a jump to a block that does not exist"))
    }

    fn value(&self, id: ValueId) -> Result<(Value, Ty), CodegenError> {
        self.values
            .get(&id.index())
            .copied()
            .ok_or_else(|| internal("an operand used before it is defined"))
    }

    fn define(&mut self, inst: &Inst, value: Value) -> Result<(), CodegenError> {
        let result = inst
            .result
            .ok_or_else(|| internal("an instruction produced a value the IR did not name"))?;
        let ty = inst
            .ty
            .ok_or_else(|| internal("a value-defining instruction with no representation"))?;
        self.values.insert(result.index(), (value, ty));
        Ok(())
    }

    // -- instructions -------------------------------------------------------

    fn emit_inst(&mut self, cur: Block, inst: &Inst) -> Result<Block, CodegenError> {
        match &inst.kind {
            InstKind::StmtMarker(stmt) => return self.emit_stmt_probe(cur, stmt.index()),
            InstKind::Safepoint => return self.emit_safepoint(cur),
            InstKind::ConstBool(v) => {
                let v = self.b.ins().iconst(types::I8, i64::from(*v));
                self.define(inst, v)?;
            }
            InstKind::ConstInt(v) => {
                let v = self.b.ins().iconst(types::I64, *v);
                self.define(inst, v)?;
            }
            InstKind::ConstUint(v) => {
                let v = self.b.ins().iconst(types::I64, v.cast_signed());
                self.define(inst, v)?;
            }
            InstKind::ConstFloat(v) => {
                let v = self.b.ins().f64const(*v);
                self.define(inst, v)?;
            }
            InstKind::ConstStr(text) => {
                let (value, next) = self.emit_const_str(cur, text.as_bytes())?;
                self.define(inst, value)?;
                return Ok(next);
            }
            InstKind::Param(index) => {
                let ty = inst
                    .ty
                    .ok_or_else(|| internal("a parameter read with no representation"))?;
                let offset = i32::try_from(*index)
                    .map_err(|_| internal("a parameter index past i32"))?
                    * VALUE_SIZE;
                let value = self.load_value(self.args_p, offset, ty)?;
                self.define(inst, value)?;
            }
            InstKind::BinOp { op, lhs, rhs } => {
                let value = self.emit_binop(*op, *lhs, *rhs)?;
                self.define(inst, value)?;
            }
            InstKind::UnOp { op, operand } => {
                let value = self.emit_unop(*op, *operand)?;
                self.define(inst, value)?;
            }
            InstKind::HelperCall { helper, args } => {
                let symbol = helper_symbol(*helper)?;
                return self.emit_helper(cur, inst, symbol, args);
            }
            // A `Core` member is native Rust behind the same ADR 0002 helper
            // entry point every runtime helper uses, so it needs no path of
            // its own here beyond naming a symbol `mwl-stdlib` registered
            // instead of one this crate's own `Helper` table does. See
            // `mwl_ir::ir::InstKind::CoreCall`.
            InstKind::CoreCall { symbol, args } => {
                return self.emit_helper(cur, inst, symbol, args);
            }
            InstKind::Call {
                target,
                receiver,
                args,
            } => {
                return self.emit_call(cur, inst, target, *receiver, args);
            }
            InstKind::New { class, ctor, args } => {
                return self.emit_new(cur, inst, class, ctor.as_deref(), args);
            }
            InstKind::FieldGet {
                object,
                class,
                field,
            } => {
                let value = self.emit_field_get(inst, *object, class, field)?;
                self.define(inst, value)?;
            }
            InstKind::FieldSet {
                object,
                class,
                field,
                value,
            } => {
                self.emit_field_set(*object, class, field, *value)?;
            }
            InstKind::InstanceOf { value, class } => {
                let result = self.emit_instanceof(*value, class)?;
                self.define(inst, result)?;
            }
            InstKind::Concat { lhs, rhs } => {
                let value = self.emit_concat(*lhs, *rhs)?;
                self.define(inst, value)?;
            }
            InstKind::ArrayNew { entries } => {
                let value = self.emit_array_new(entries)?;
                self.define(inst, value)?;
            }
            InstKind::ArrayGet { array, key } => {
                let value = self.emit_array_get(inst, *array, *key)?;
                self.define(inst, value)?;
            }
            InstKind::ArraySet { array, key, value } => {
                let result = self.emit_array_write(
                    "mwl_array_set",
                    RuntimeSig::ArraySet,
                    *array,
                    Some(*key),
                    *value,
                )?;
                self.define(inst, result)?;
            }
            InstKind::ArrayAppend { array, value } => {
                let result = self.emit_array_write(
                    "mwl_array_append",
                    RuntimeSig::ArrayAppend,
                    *array,
                    None,
                    *value,
                )?;
                self.define(inst, result)?;
            }
            InstKind::ArrayUnset { array, key } => {
                let (array, _) = self.value(*array)?;
                let (key, _) = self.value(*key)?;
                let callee = self.runtime_ref("mwl_array_unset", RuntimeSig::ArrayAppend)?;
                let call = self.b.ins().call(callee, &[array, key]);
                let result = self.b.inst_results(call)[0];
                self.define(inst, result)?;
            }
            InstKind::ArrayNextSlot { array, from } => {
                let (array, _) = self.value(*array)?;
                let (from, _) = self.value(*from)?;
                let callee = self.runtime_ref("mwl_array_next_slot", RuntimeSig::ArrayNextSlot)?;
                let call = self.b.ins().call(callee, &[array, from]);
                let result = self.b.inst_results(call)[0];
                self.define(inst, result)?;
            }
            InstKind::ArrayKeyAt { array, slot } => {
                let (array, _) = self.value(*array)?;
                let (slot, _) = self.value(*slot)?;
                let callee = self.runtime_ref("mwl_array_key_at", RuntimeSig::ArrayKeyAt)?;
                let call = self.b.ins().call(callee, &[array, slot]);
                let result = self.b.inst_results(call)[0];
                self.define(inst, result)?;
            }
            InstKind::ArrayValueAt { array, slot } => {
                let ty = inst
                    .ty
                    .ok_or_else(|| internal("a foreach value binding with no representation"))?;
                let (array, _) = self.value(*array)?;
                let (slot, _) = self.value(*slot)?;
                let out = self.value_slot();
                let callee = self.runtime_ref("mwl_array_value_at", RuntimeSig::ArrayValueAt)?;
                self.b.ins().call(callee, &[array, slot, out]);
                let value = self.load_value(out, 0, ty)?;
                self.define(inst, value)?;
            }
            InstKind::Retain { operand } => {
                let (value, ty) = self.value(*operand)?;
                self.emit_refcount(true, value, ty)?;
            }
            InstKind::Release { operand } => {
                let (value, ty) = self.value(*operand)?;
                self.emit_refcount(false, value, ty)?;
            }
            InstKind::TakeThrown => {
                let callee = self.runtime_ref("mwl_take_thrown", RuntimeSig::PtrToPtr)?;
                let call = self.b.ins().call(callee, &[self.ctx_p]);
                let value = self.b.inst_results(call)[0];
                self.define(inst, value)?;
            }
            InstKind::Throwable { op, operand } => {
                let (value, _) = self.value(*operand)?;
                let symbol = match op {
                    ThrowableOp::New => "mwl_exception_new",
                    ThrowableOp::Message => "mwl_throwable_message",
                    ThrowableOp::TraceAsString => "mwl_throwable_trace",
                    other => {
                        return Err(CodegenError::Unsupported(format!(
                            "the exception operation {other:?}"
                        )));
                    }
                };
                let callee = self.runtime_ref(symbol, RuntimeSig::PtrToPtr)?;
                let call = self.b.ins().call(callee, &[value]);
                let result = self.b.inst_results(call)[0];
                self.define(inst, result)?;
            }
            InstKind::Phi { .. } => return Err(internal("a phi reached the instruction walk")),
            other => {
                return Err(CodegenError::Unsupported(describe(other)));
            }
        }
        Ok(cur)
    }

    /// The safepoint poll: one load of [`mwl_runtime::Ctx`]'s first word, one
    /// predicted-not-taken branch, and an out-of-line call to
    /// [`mwl_runtime::mwl_safepoint`] whose status is checked like any other.
    fn emit_safepoint(&mut self, _cur: Block) -> Result<Block, CodegenError> {
        let offset = i32::try_from(SAFEPOINT_OFFSET)
            .map_err(|_| internal("the safepoint word sits past a 2 GiB offset"))?;
        let flags = self
            .b
            .ins()
            .load(types::I64, ctx_word(), self.ctx_p, offset);

        let slow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(flags, slow, &[], cont, &[]);

        self.b.switch_to_block(slow);
        let callee = self.runtime_ref("mwl_safepoint", RuntimeSig::Safepoint)?;
        let call = self.b.ins().call(callee, &[self.ctx_p]);
        let status = self.b.inst_results(call)[0];
        let stop = self.b.create_block();
        self.b.ins().brif(status, stop, &[], cont, &[]);

        self.b.switch_to_block(stop);
        self.b.ins().return_(&[status]);

        self.b.switch_to_block(cont);
        Ok(cont)
    }

    /// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// § 1's statement-boundary probe: the identical load-and-branch shape as
    /// [`Self::emit_safepoint`], against the *second* hot word, with no status
    /// to check because [`mwl_runtime::mwl_probe_stmt`] cannot fail.
    ///
    /// Emitted unconditionally, in every compiled unit, whether or not any
    /// request ever sets a bit — that is the entire mechanism, and the reason
    /// coverage can be switched on for a request already running.
    fn emit_stmt_probe(&mut self, _cur: Block, stmt: u32) -> Result<Block, CodegenError> {
        let offset = i32::try_from(DEBUG_FLAGS_OFFSET)
            .map_err(|_| internal("the debug-flags word sits past a 2 GiB offset"))?;
        let flags = self
            .b
            .ins()
            .load(types::I64, ctx_word(), self.ctx_p, offset);

        let slow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(flags, slow, &[], cont, &[]);

        self.b.switch_to_block(slow);
        let callee = self.runtime_ref("mwl_probe_stmt", RuntimeSig::Probe)?;
        let id = self.b.ins().iconst(types::I32, i64::from(stmt));
        self.b.ins().call(callee, &[self.ctx_p, id]);
        self.b.ins().jump(cont, &[]);

        self.b.switch_to_block(cont);
        Ok(cont)
    }

    /// A string literal: its bytes go into the unit's data section, and
    /// `mwl_str_new` copies them into a fresh refcounted allocation.
    ///
    /// The copy is the crate docs' known gap 4 — an immortal header with a
    /// pinned refcount would make this a constant with no call at all — and it
    /// is deliberately not fixed here: `mwl-runtime` owns `StrHeader`'s
    /// layout, so pinning a refcount is that crate's decision to make, not a
    /// pattern this one should start writing into a data section on its own.
    fn emit_const_str(&mut self, cur: Block, bytes: &[u8]) -> Result<(Value, Block), CodegenError> {
        let (address, len) = self.emit_bytes(bytes)?;
        let callee = self.runtime_ref("mwl_str_new", RuntimeSig::StrNew)?;
        let call = self.b.ins().call(callee, &[address, len]);
        let value = self.b.inst_results(call)[0];
        Ok((value, cur))
    }

    /// Puts `bytes` in the unit's data section and materializes its address
    /// and length as two values — the shape every runtime primitive taking
    /// static bytes wants (`mwl_str_new`, and ADR 0018's call-site probes).
    fn emit_bytes(&mut self, bytes: &[u8]) -> Result<(Value, Value), CodegenError> {
        let name = format!("mwl_bytes_{}", *self.literals);
        *self.literals += 1;

        let mut desc = DataDescription::new();
        // A zero-length literal still needs a real address to hand to its
        // consumer, which ignores the pointer when the length is zero but is
        // handed one regardless; a one-byte object is the cheapest way to keep
        // the two cases identical here.
        desc.define(if bytes.is_empty() {
            Box::new([0_u8]) as Box<[u8]>
        } else {
            bytes.to_vec().into_boxed_slice()
        });
        let data = self
            .module
            .declare_data(&name, Linkage::Local, false, false)
            .map_err(|source| CodegenError::Cranelift {
                function: self.f.name.clone(),
                source: Box::new(source),
            })?;
        self.module
            .define_data(data, &desc)
            .map_err(|source| CodegenError::Cranelift {
                function: self.f.name.clone(),
                source: Box::new(source),
            })?;

        let global = self.module.declare_data_in_func(data, self.b.func);
        let address = self.b.ins().symbol_value(types::I64, global);
        let len = self.b.ins().iconst(
            types::I64,
            i64::try_from(bytes.len()).map_err(|_| internal("a literal past i64 bytes"))?,
        );
        Ok((address, len))
    }

    fn emit_binop(&mut self, op: BinOp, lhs: ValueId, rhs: ValueId) -> Result<Value, CodegenError> {
        let (l, ty) = self.value(lhs)?;
        let (r, rty) = self.value(rhs)?;
        if ty != rty {
            return Err(internal(
                "a binary operator over mismatched representations",
            ));
        }

        let signed = matches!(ty, Ty::Int);
        let float = matches!(ty, Ty::Float);
        let integral = matches!(ty, Ty::Int | Ty::Uint | Ty::Bool);
        if !float && !integral {
            return Err(CodegenError::Unsupported(format!(
                "a `{op:?}` over representation {ty:?}"
            )));
        }

        let value = match op {
            BinOp::Add if float => self.b.ins().fadd(l, r),
            BinOp::Sub if float => self.b.ins().fsub(l, r),
            BinOp::Mul if float => self.b.ins().fmul(l, r),
            BinOp::Div if float => self.b.ins().fdiv(l, r),
            // Wrapping, for now: ADR 0007 § 4 makes integer overflow a throw
            // rather than a silent widening to `float`, and there is no error
            // edge in the IR to branch to yet (crate docs, known gap 3). The
            // divergence is a wrong *value* in a case PHP would also not
            // produce, which is why these are emitted while `Div`/`Mod` below
            // are not.
            BinOp::Add => self.b.ins().iadd(l, r),
            BinOp::Sub => self.b.ins().isub(l, r),
            BinOp::Mul => self.b.ins().imul(l, r),
            // Deliberately *not* emitted, and not for the same reason as the
            // overflow gap above: `sdiv`/`udiv` **trap** on a zero divisor,
            // and a trap takes the whole process down. That is a request-
            // isolation failure — CLAUDE.md's priority 1 — not a wrong answer,
            // so it waits for the throw path rather than shipping ahead of it.
            BinOp::Div | BinOp::Mod => {
                return Err(CodegenError::Unsupported(format!(
                    "integer `{op:?}`, whose zero divisor must throw \
                     (ADR 0007 § 4) rather than trap the process"
                )));
            }
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                if float {
                    let cc = match op {
                        BinOp::Eq => FloatCC::Equal,
                        BinOp::NotEq => FloatCC::NotEqual,
                        BinOp::Lt => FloatCC::LessThan,
                        BinOp::LtEq => FloatCC::LessThanOrEqual,
                        BinOp::Gt => FloatCC::GreaterThan,
                        _ => FloatCC::GreaterThanOrEqual,
                    };
                    self.b.ins().fcmp(cc, l, r)
                } else {
                    let cc = match (op, signed) {
                        (BinOp::Eq, _) => IntCC::Equal,
                        (BinOp::NotEq, _) => IntCC::NotEqual,
                        (BinOp::Lt, true) => IntCC::SignedLessThan,
                        (BinOp::Lt, false) => IntCC::UnsignedLessThan,
                        (BinOp::LtEq, true) => IntCC::SignedLessThanOrEqual,
                        (BinOp::LtEq, false) => IntCC::UnsignedLessThanOrEqual,
                        (BinOp::Gt, true) => IntCC::SignedGreaterThan,
                        (BinOp::Gt, false) => IntCC::UnsignedGreaterThan,
                        (_, true) => IntCC::SignedGreaterThanOrEqual,
                        (_, false) => IntCC::UnsignedGreaterThanOrEqual,
                    };
                    self.b.ins().icmp(cc, l, r)
                }
            }
            other => {
                return Err(CodegenError::Unsupported(format!(
                    "the binary operator {other:?}"
                )));
            }
        };
        Ok(value)
    }

    fn emit_unop(&mut self, op: UnOp, operand: ValueId) -> Result<Value, CodegenError> {
        let (v, ty) = self.value(operand)?;
        Ok(match (op, ty) {
            (UnOp::Neg, Ty::Float) => self.b.ins().fneg(v),
            // Wrapping at `i64::MIN`, the same known gap the additive
            // operators carry above — never a trap, so it does not join
            // `Div`/`Mod` on the refused list.
            (UnOp::Neg, Ty::Int | Ty::Uint) => self.b.ins().ineg(v),
            (UnOp::Not, Ty::Bool) => {
                let zero = self.b.ins().iconst(types::I8, 0);
                self.b.ins().icmp(IntCC::Equal, v, zero)
            }
            (op, ty) => {
                return Err(CodegenError::Unsupported(format!(
                    "the unary operator {op:?} over representation {ty:?}"
                )));
            }
        })
    }

    /// One runtime helper call, in ADR 0002's shape: the arguments
    /// materialized into a stack slot of 16-byte [`mwl_runtime::Value`]s, a
    /// second slot for the result, and the status check after.
    fn emit_helper(
        &mut self,
        cur: Block,
        inst: &Inst,
        symbol: &'static str,
        args: &[ValueId],
    ) -> Result<Block, CodegenError> {
        let count =
            i32::try_from(args.len()).map_err(|_| internal("a helper call past i32 args"))?;
        let args_p = if args.is_empty() {
            // `mwl_runtime::run_helper` reads no argument slice at arity zero,
            // and splits that case out precisely so a null pointer is legal.
            self.b.ins().iconst(types::I64, 0)
        } else {
            let slot = self.b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (count * VALUE_SIZE).cast_unsigned(),
                VALUE_ALIGN_SHIFT,
            ));
            let base = self.b.ins().stack_addr(types::I64, slot, 0);
            for (index, arg) in args.iter().enumerate() {
                let (value, ty) = self.value(*arg)?;
                let offset = i32::try_from(index)
                    .map_err(|_| internal("a helper call past i32 args"))?
                    * VALUE_SIZE;
                self.store_value(base, offset, value, ty)?;
            }
            base
        };

        let out_slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            VALUE_SIZE.cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        let out_p = self.b.ins().stack_addr(types::I64, out_slot, 0);

        let callee = self.runtime_ref(symbol, RuntimeSig::Helper)?;
        let call = self.b.ins().call(callee, &[self.ctx_p, args_p, out_p]);
        let status = self.b.inst_results(call)[0];
        let cont = self.emit_status_check(status, inst.on_error)?;

        if let Some(ty) = inst.ty {
            let value = self.load_value(out_p, 0, ty)?;
            self.define(inst, value)?;
        }
        let _ = cur;
        Ok(cont)
    }

    /// One MWL-level call, in ADR 0002's shape.
    ///
    /// Structurally identical to [`Self::emit_helper`] — arguments
    /// materialized into a stack slot of 16-byte [`mwl_runtime::Value`]s, a
    /// second slot for the result, the compare-and-branch on the returned
    /// status — and deliberately so: ADR 0002 makes one calling convention
    /// normative for *every* call, so a runtime helper and a compiled MWL
    /// method differ here only in which `FuncRef` is called.
    ///
    /// # The receiver slot
    ///
    /// `mwl_ir::lower::lower_method` gives every lowered method an implicit
    /// receiver at parameter index 0, whether or not its body reads `$this`
    /// (see `Function::params`' own doc comment). So argument slot 0 always
    /// exists: an instance call stores its receiver there like any other
    /// argument, and a static call — which has no receiver value at all —
    /// fills it with `null`.
    ///
    /// Ownership follows the same convention every argument does: the caller
    /// retains an aliasing receiver and the callee releases it at scope exit.
    /// `mwl_ir::lower`'s `MethodCall` arm inserts that retain, so nothing here
    /// does.
    fn emit_call(
        &mut self,
        cur: Block,
        inst: &Inst,
        target: &str,
        receiver: Option<ValueId>,
        args: &[ValueId],
    ) -> Result<Block, CodegenError> {
        let receiver = receiver.map(|id| self.value(id)).transpose()?;
        let (cont, out_p) = self.emit_invoke(inst, target, receiver, args)?;
        if let Some(ty) = inst.ty.filter(|ty| !matches!(ty, Ty::Void)) {
            let value = self.load_value(out_p, 0, ty)?;
            self.define(inst, value)?;
        }
        let _ = cur;
        Ok(cont)
    }

    /// The call itself, shared by [`Self::emit_call`] and the constructor
    /// invocation inside [`Self::emit_new`]: arguments materialized, both
    /// probes emitted, the status checked. Returns the block execution
    /// continues in and the `out` slot the callee wrote its result into —
    /// which caller decides what to do with, since `new`'s own result is the
    /// instance rather than anything the constructor returned.
    fn emit_invoke(
        &mut self,
        inst: &Inst,
        target: &str,
        receiver: Option<(Value, Ty)>,
        args: &[ValueId],
    ) -> Result<(Block, Value), CodegenError> {
        let callee = self.callee_ref(target)?;
        let label = self.emit_bytes(target.as_bytes())?;
        self.emit_call_probe("mwl_probe_call_enter", RuntimeSig::ProbeCall, label, None)?;

        // One slot per argument, plus the implicit receiver at index 0.
        let count =
            i32::try_from(args.len() + 1).map_err(|_| internal("a call past i32 arguments"))?;
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (count * VALUE_SIZE).cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        let args_p = self.b.ins().stack_addr(types::I64, slot, 0);
        match receiver {
            Some((value, ty)) => self.store_value(args_p, 0, value, ty)?,
            None => self.store_tag_and_bits(args_p, 0, Tag::Null, None)?,
        }
        for (index, arg) in args.iter().enumerate() {
            let (value, ty) = self.value(*arg)?;
            let offset = i32::try_from(index + 1)
                .map_err(|_| internal("a call past i32 arguments"))?
                * VALUE_SIZE;
            self.store_value(args_p, offset, value, ty)?;
        }

        let out_slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            VALUE_SIZE.cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        let out_p = self.b.ins().stack_addr(types::I64, out_slot, 0);

        let call = self.b.ins().call(callee, &[self.ctx_p, args_p, out_p]);
        let status = self.b.inst_results(call)[0];
        // Before the status check, so a thrown or `FATAL` exit is traced as it
        // happened rather than skipped along with the rest of the frame.
        self.emit_call_probe(
            "mwl_probe_call_exit",
            RuntimeSig::ProbeCallExit,
            label,
            Some(status),
        )?;
        let cont = self.emit_status_check(status, inst.on_error)?;
        Ok((cont, out_p))
    }

    /// `new Foo(...)`: allocate the instance, then invoke its resolved
    /// constructor on it.
    ///
    /// The IR bundles the two into one instruction
    /// (`mwl_ir::ir::InstKind::New`), so the ownership bookkeeping between
    /// them is this function's rather than lowering's:
    /// `mwl_object_new` returns the one reference the `New` *result* owns, and
    /// the constructor — an ordinary method whose frame releases every
    /// refcounted parameter at scope exit — needs a reference of its own. So
    /// the receiver is retained before the call, exactly the retain
    /// `mwl_ir::lower` inserts at an ordinary `$obj->m()` site.
    ///
    /// The descriptor address is an `iconst`: see [`crate::Classes`] for why a
    /// JIT can bake one in.
    fn emit_new(
        &mut self,
        cur: Block,
        inst: &Inst,
        class: &str,
        ctor: Option<&str>,
        args: &[ValueId],
    ) -> Result<Block, CodegenError> {
        let desc = self.classes.desc(class).ok_or_else(|| {
            CodegenError::Unsupported(format!(
                "`new {class}(...)`, whose class this unit declares no layout for"
            ))
        })?;
        let address = i64::try_from(desc.addr())
            .map_err(|_| internal("a class descriptor above i64::MAX"))?;
        let desc = self.b.ins().iconst(types::I64, address);
        let callee = self.runtime_ref("mwl_object_new", RuntimeSig::PtrToPtr)?;
        let call = self.b.ins().call(callee, &[desc]);
        let object = self.b.inst_results(call)[0];
        self.define(inst, object)?;

        let Some(ctor) = ctor else {
            // No constructor anywhere in the chain — `mwl_ir::lower` already
            // asserted the call site passed no arguments, so allocation is the
            // whole of `new`. Every slot is `null`, which ADR 0022 makes
            // unobservable.
            return Ok(cur);
        };
        let retain = self.runtime_ref("mwl_object_retain", RuntimeSig::Refcount)?;
        self.b.ins().call(retain, &[object]);
        let (cont, _out) = self.emit_invoke(inst, ctor, Some((object, Ty::Object)), args)?;
        Ok(cont)
    }

    /// `$obj instanceof Class`: one call, with the descriptor address baked in
    /// exactly the way [`Self::emit_new`] bakes the allocated class's.
    ///
    /// Nothing is retained: the receiver is only read, the way a `FieldGet`
    /// reads its own.
    fn emit_instanceof(&mut self, value: ValueId, class: &str) -> Result<Value, CodegenError> {
        let desc = self.classes.desc(class).ok_or_else(|| {
            CodegenError::Unsupported(format!(
                "`instanceof {class}`, whose class this unit declares no descriptor for"
            ))
        })?;
        let address = i64::try_from(desc.addr())
            .map_err(|_| internal("a class descriptor above i64::MAX"))?;
        let desc = self.b.ins().iconst(types::I64, address);
        let (object, _) = self.value(value)?;
        let callee = self.runtime_ref("mwl_object_instanceof", RuntimeSig::InstanceOf)?;
        let call = self.b.ins().call(callee, &[object, desc]);
        Ok(self.b.inst_results(call)[0])
    }

    /// `$obj->prop`: one load out of the receiver's field slot.
    ///
    /// The slot's *offset* comes from [`mwl_runtime::field_offset`], so this
    /// crate never restates the object layout — and the tag is not re-read,
    /// because the field's static type is already settled (see
    /// [`Self::load_value`]'s own note, which this shares).
    ///
    /// Nothing is retained here. `mwl_ir::ir::InstKind::FieldGet` reads the
    /// field without taking ownership, and `mwl_ir::lower::is_aliasing_read`
    /// makes the consumer insert the retain if it keeps the value.
    fn emit_field_get(
        &mut self,
        inst: &Inst,
        object: ValueId,
        class: &str,
        field: &str,
    ) -> Result<Value, CodegenError> {
        let offset = self.field_offset(class, field)?;
        let (base, _) = self.value(object)?;
        let ty = inst
            .ty
            .ok_or_else(|| internal("a property read with no representation"))?;
        self.load_value(base, offset, ty)
    }

    /// `$obj->prop = expr;`: one store into the receiver's field slot.
    ///
    /// A plain store, with no release of what the slot held:
    /// `mwl_ir::lower::lower_reassignment` already emitted the `FieldGet` and
    /// `Release` pair for the previous value, ahead of this instruction. That
    /// split is why this is not `mwl_runtime::mwl_object_field_set`, which
    /// releases for its caller.
    fn emit_field_set(
        &mut self,
        object: ValueId,
        class: &str,
        field: &str,
        value: ValueId,
    ) -> Result<(), CodegenError> {
        let offset = self.field_offset(class, field)?;
        let (base, _) = self.value(object)?;
        let (value, ty) = self.value(value)?;
        self.store_value(base, offset, value, ty)
    }

    /// The byte offset of `class::field` within an instance, as an `i32`
    /// Cranelift memory operand.
    fn field_offset(&self, class: &str, field: &str) -> Result<i32, CodegenError> {
        let slot = self.classes.slot(class, field).ok_or_else(|| {
            CodegenError::Unsupported(format!(
                "the property `{class}::{field}`, which this unit declares no slot for"
            ))
        })?;
        i32::try_from(mwl_runtime::field_offset(slot))
            .map_err(|_| internal("an object field sitting past a 2 GiB offset"))
    }

    /// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// § 1's call-site probe, emitted twice per call: once before and once
    /// after, `status` distinguishing them.
    ///
    /// Exactly [`Self::emit_stmt_probe`]'s shape — one load of the debug-flags
    /// word, one predicted-not-taken branch, an out-of-line call that never
    /// runs while every bit is off — because it is the same mechanism at a
    /// different site, and ADR 0018's whole argument is that both are one
    /// cost class rather than a second compiled tier.
    ///
    /// The word is re-read at the exit site rather than the entry site's load
    /// being reused: a request may turn tracing on or off *during* the call,
    /// and `mwl_probe_call_exit`'s own doc comment says why recording that
    /// honestly beats a balanced-looking reconstruction.
    fn emit_call_probe(
        &mut self,
        symbol: &'static str,
        sig: RuntimeSig,
        label: (Value, Value),
        status: Option<Value>,
    ) -> Result<(), CodegenError> {
        let offset = i32::try_from(DEBUG_FLAGS_OFFSET)
            .map_err(|_| internal("the debug-flags word sits past a 2 GiB offset"))?;
        let flags = self
            .b
            .ins()
            .load(types::I64, ctx_word(), self.ctx_p, offset);

        let slow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(flags, slow, &[], cont, &[]);

        self.b.switch_to_block(slow);
        let probe = self.runtime_ref(symbol, sig)?;
        let (address, len) = label;
        match status {
            None => {
                self.b.ins().call(probe, &[self.ctx_p, address, len]);
            }
            Some(status) => {
                self.b
                    .ins()
                    .call(probe, &[self.ctx_p, address, len, status]);
            }
        }
        self.b.ins().jump(cont, &[]);

        self.b.switch_to_block(cont);
        Ok(())
    }

    /// A [`codegen::ir::FuncRef`] for one of the unit's own functions, cached
    /// per emitted function the same way [`Self::runtime_ref`] caches an
    /// import.
    fn callee_ref(&mut self, target: &str) -> Result<codegen::ir::FuncRef, CodegenError> {
        if let Some(reference) = self.callee_refs.get(target) {
            return Ok(*reference);
        }
        let id = *self
            .functions
            .get(target)
            .ok_or_else(|| CodegenError::UnknownTarget {
                caller: self.f.name.clone(),
                target: target.to_owned(),
            })?;
        let reference = self.module.declare_func_in_func(id, self.b.func);
        self.callee_refs.insert(target.to_owned(), reference);
        Ok(reference)
    }

    /// `.` concatenation: one call to `mwl_str_concat`, which allocates the
    /// joined buffer once.
    ///
    /// No status check and no `Value` materialization: like `mwl_str_new`,
    /// this is a memory primitive over bare `StrHeader` pointers rather than
    /// an ADR 0002 helper, because it cannot fail — see `mwl-runtime`'s
    /// "primitives compiled code calls" section for that split. Neither
    /// operand is retained or released here; `mwl_ir::ir::InstKind::Concat`'s
    /// own doc comment owns that rule and `mwl-ir` emits the releases.
    fn emit_concat(&mut self, lhs: ValueId, rhs: ValueId) -> Result<Value, CodegenError> {
        let (l, lty) = self.value(lhs)?;
        let (r, rty) = self.value(rhs)?;
        for ty in [lty, rty] {
            if !matches!(ty, Ty::Str | Ty::Bytes) {
                return Err(internal(
                    "a concatenation operand that lowering left unconverted",
                ));
            }
        }
        let callee = self.runtime_ref("mwl_str_concat", RuntimeSig::StrConcat)?;
        let call = self.b.ins().call(callee, &[l, r]);
        Ok(self.b.inst_results(call)[0])
    }

    /// A 16-byte stack slot and its address — the one shape a [`MwlValue`]
    /// crosses a runtime-primitive boundary in, since a struct that size is
    /// classified differently by the SysV and Windows x64 ABIs (see
    /// `mwl_runtime::array`'s own note).
    fn value_slot(&mut self) -> Value {
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            VALUE_SIZE.cast_unsigned(),
            VALUE_ALIGN_SHIFT,
        ));
        self.b.ins().stack_addr(types::I64, slot, 0)
    }

    /// `[...]`: one allocation, then one write per entry.
    ///
    /// `mwl_ir::ir::InstKind::ArrayNew` carries each key as a decimal string
    /// computed at lowering time, so each one is emitted here exactly like a
    /// `ConstStr` — a fresh allocation whose single reference transfers
    /// straight into the array, which is why no retain accompanies it. Each
    /// write yields the array the next one writes into, per that instruction's
    /// consume-one-reference-yield-one protocol; the pointer never actually
    /// changes here, because a literal under construction is solely owned, but
    /// threading it is what keeps this on the one protocol rather than beside
    /// it.
    fn emit_array_new(&mut self, entries: &[(String, ValueId)]) -> Result<Value, CodegenError> {
        let callee = self.runtime_ref("mwl_array_new", RuntimeSig::ArrayNew)?;
        let call = self.b.ins().call(callee, &[]);
        let mut array = self.b.inst_results(call)[0];

        for (key, value) in entries {
            let (address, len) = self.emit_bytes(key.as_bytes())?;
            let new_str = self.runtime_ref("mwl_str_new", RuntimeSig::StrNew)?;
            let call = self.b.ins().call(new_str, &[address, len]);
            let key = self.b.inst_results(call)[0];

            let slot = self.value_slot();
            let (value, ty) = self.value(*value)?;
            self.store_value(slot, 0, value, ty)?;

            let set = self.runtime_ref("mwl_array_set", RuntimeSig::ArraySet)?;
            let call = self.b.ins().call(set, &[array, key, slot]);
            array = self.b.inst_results(call)[0];
        }
        Ok(array)
    }

    /// `$a[$k]`: one call, reading the result back out of a stack slot.
    ///
    /// Nothing is retained here. `mwl_ir::ir::InstKind::ArrayGet` reads the
    /// element without taking ownership, exactly like a `FieldGet`, and
    /// `mwl_ir::lower::is_aliasing_read` makes the consumer insert the retain
    /// if it keeps the value.
    fn emit_array_get(
        &mut self,
        inst: &Inst,
        array: ValueId,
        key: ValueId,
    ) -> Result<Value, CodegenError> {
        let ty = inst
            .ty
            .ok_or_else(|| internal("an array read with no representation"))?;
        let (array, _) = self.value(array)?;
        let (key, _) = self.value(key)?;
        let out = self.value_slot();
        let callee = self.runtime_ref("mwl_array_get", RuntimeSig::ArrayGet)?;
        self.b.ins().call(callee, &[array, key, out]);
        self.load_value(out, 0, ty)
    }

    /// `$a[$k] = expr;` and `$a[] = expr;`: one call that consumes the array
    /// and yields the array that now holds the entry.
    ///
    /// No refcount operation of any kind. `mwl_ir::ir::InstKind::ArraySet`'s
    /// own doc comment owns that rule: the reference the primitive consumes
    /// and the one it yields are the holder's same one slot, and `mwl-ir`
    /// already emitted whatever retain the key and value needed.
    fn emit_array_write(
        &mut self,
        symbol: &'static str,
        sig: RuntimeSig,
        array: ValueId,
        key: Option<ValueId>,
        value: ValueId,
    ) -> Result<Value, CodegenError> {
        let (array, _) = self.value(array)?;
        let key = key.map(|k| self.value(k)).transpose()?.map(|(v, _)| v);
        let slot = self.value_slot();
        let (value, ty) = self.value(value)?;
        self.store_value(slot, 0, value, ty)?;

        let callee = self.runtime_ref(symbol, sig)?;
        let call = match key {
            Some(key) => self.b.ins().call(callee, &[array, key, slot]),
            None => self.b.ins().call(callee, &[array, slot]),
        };
        Ok(self.b.inst_results(call)[0])
    }

    /// A retain or release of one refcounted value.
    ///
    /// [`Ty::Str`] and [`Ty::Bytes`] share the `StrHeader` representation, so
    /// they share the primitive.
    ///
    /// An object's or an array's release is where a whole graph can be freed
    /// at once — `mwl_runtime::release`'s own docs explain why that sweep is
    /// one iterative worklist shared by both, which is what keeps this a
    /// single call rather than a depth-bounded one.
    fn emit_refcount(&mut self, retain: bool, value: Value, ty: Ty) -> Result<(), CodegenError> {
        let symbol = match (ty, retain) {
            (Ty::Str | Ty::Bytes, true) => "mwl_str_retain",
            (Ty::Str | Ty::Bytes, false) => "mwl_str_release",
            (Ty::Object, true) => "mwl_object_retain",
            (Ty::Object, false) => "mwl_object_release",
            (Ty::Array, true) => "mwl_array_retain",
            (Ty::Array, false) => "mwl_array_release",
            (Ty::Throwable, true) => "mwl_throwable_retain",
            (Ty::Throwable, false) => "mwl_throwable_release",
            (other, _) => {
                return Err(CodegenError::Unsupported(format!(
                    "a refcount operation on representation {other:?}"
                )));
            }
        };
        let callee = self.runtime_ref(symbol, RuntimeSig::Refcount)?;
        self.b.ins().call(callee, &[value]);
        Ok(())
    }

    /// ADR 0002's compare-and-branch: on a non-`OK` status, take the
    /// instruction's error edge; otherwise carry on in a fresh block.
    ///
    /// `on_error` is [`mwl_ir::ir::Inst::on_error`] — the landing block the IR
    /// built for this exact program point, carrying the frame's cleanup and
    /// the decision between propagating and entering a `catch`. The status
    /// travels there as that block's one parameter.
    ///
    /// `None` keeps the pre-error-edge shape: return the status onward from a
    /// bare fail block, releasing nothing. That is reached only by the
    /// instructions whose sole non-`OK` outcome is a `FATAL` — see
    /// `Inst::on_error`'s own doc comment, which states that consequence
    /// rather than hiding it.
    fn emit_status_check(
        &mut self,
        status: Value,
        on_error: Option<BlockId>,
    ) -> Result<Block, CodegenError> {
        let failed = self
            .b
            .ins()
            .icmp_imm_s(IntCC::NotEqual, status, i64::from(OK));
        let cont = self.b.create_block();
        match on_error {
            Some(landing) => {
                let target = self.block(landing)?;
                let args = [codegen::ir::BlockArg::Value(status)];
                self.b.ins().brif(failed, target, &args, cont, &[]);
            }
            None => {
                let fail = self.b.create_block();
                self.b.ins().brif(failed, fail, &[], cont, &[]);
                self.b.switch_to_block(fail);
                self.b.ins().return_(&[status]);
            }
        }
        self.b.switch_to_block(cont);
        Ok(cont)
    }

    // -- terminators --------------------------------------------------------

    fn emit_terminator(&mut self, cur: Block, block: &BasicBlock) -> Result<(), CodegenError> {
        match &block.term {
            Terminator::Return(None) => {
                self.store_tag_and_bits(self.out_p, 0, Tag::Null, None)?;
                let ok = self.b.ins().iconst(types::I32, i64::from(OK));
                self.b.ins().return_(&[ok]);
            }
            Terminator::Return(Some(id)) => {
                let (value, ty) = self.value(*id)?;
                self.store_value(self.out_p, 0, value, ty)?;
                let ok = self.b.ins().iconst(types::I32, i64::from(OK));
                self.b.ins().return_(&[ok]);
            }
            Terminator::Jump(target) => {
                let args = self.phi_args(block.id, *target)?;
                let target = self.block(*target)?;
                self.b.ins().jump(target, &args);
            }
            Terminator::Branch {
                cond,
                then_block,
                else_block,
                ..
            } => {
                let (cond, _) = self.value(*cond)?;
                let then_args = self.phi_args(block.id, *then_block)?;
                let else_args = self.phi_args(block.id, *else_block)?;
                let then_target = self.block(*then_block)?;
                let else_target = self.block(*else_block)?;
                self.b
                    .ins()
                    .brif(cond, then_target, &then_args, else_target, &else_args);
            }
            Terminator::Throw { value, landing } => {
                let (thrown, ty) = self.value(*value)?;
                if !matches!(ty, Ty::Throwable) {
                    return Err(internal("a `throw` of something that is not an exception"));
                }
                // Ownership of the exception transfers to the context here —
                // `mwl_ir::lower` already retained an aliasing operand.
                let callee = self.runtime_ref("mwl_raise", RuntimeSig::Raise)?;
                self.b.ins().call(callee, &[self.ctx_p, thrown]);
                let status = self.b.ins().iconst(types::I32, i64::from(THROWN));
                let target = self.block(*landing)?;
                self.b
                    .ins()
                    .jump(target, &[codegen::ir::BlockArg::Value(status)]);
            }
            Terminator::Propagate { frame } => {
                let status = self.landing_status()?;
                let (address, len) = self.emit_bytes(frame.as_bytes())?;
                let callee = self.runtime_ref("mwl_trace_push", RuntimeSig::ProbeCallExit)?;
                self.b
                    .ins()
                    .call(callee, &[self.ctx_p, address, len, status]);
                self.b.ins().return_(&[status]);
            }
            Terminator::Catch { handler } => {
                let status = self.landing_status()?;
                // Only a `THROWN` is catchable: ADR 0020 keeps a `FATAL` out
                // of every `catch`, at the type level in the language and by
                // this comparison in the generated code.
                let caught = self
                    .b
                    .ins()
                    .icmp_imm_s(IntCC::Equal, status, i64::from(THROWN));
                let args = self.phi_args(block.id, *handler)?;
                let target = self.block(*handler)?;
                let onward = self.b.create_block();
                self.b.ins().brif(caught, target, &args, onward, &[]);
                self.b.switch_to_block(onward);
                self.b.ins().return_(&[status]);
            }
            other => {
                return Err(CodegenError::Unsupported(format!(
                    "the terminator {other:?}"
                )));
            }
        }
        let _ = cur;
        Ok(())
    }

    /// The status parameter of the landing block being emitted.
    fn landing_status(&self) -> Result<Value, CodegenError> {
        self.landing_status
            .ok_or_else(|| internal("a landing terminator outside a landing block"))
    }

    /// The values `pred`'s jump into `succ` supplies for `succ`'s phis, in
    /// block-parameter order.
    ///
    /// A phi's `incoming` list is keyed by the *IR* block a value arrived
    /// from, which is the block whose terminator this is — not the Cranelift
    /// block that terminator physically ends up in, since a status check or a
    /// flag probe may have split it.
    fn phi_args(
        &self,
        pred: BlockId,
        succ: BlockId,
    ) -> Result<Vec<codegen::ir::BlockArg>, CodegenError> {
        let count = *self
            .phi_counts
            .get(&succ.index())
            .ok_or_else(|| internal("a jump to a block with no recorded phi count"))?;
        let block = self
            .f
            .blocks
            .iter()
            .find(|b| b.id.index() == succ.index())
            .ok_or_else(|| internal("a jump to a block that does not exist"))?;

        let mut args = Vec::with_capacity(count);
        for inst in &block.insts[..count] {
            let InstKind::Phi { incoming } = &inst.kind else {
                return Err(internal("a non-phi in a block's leading phi run"));
            };
            let (_, id) = incoming
                .iter()
                .find(|(from, _)| from.index() == pred.index())
                .ok_or_else(|| internal("a phi with no incoming value for a real predecessor"))?;
            let (value, _) = self.value(*id)?;
            args.push(codegen::ir::BlockArg::Value(value));
        }
        Ok(args)
    }

    // -- the 16-byte Value boundary -----------------------------------------

    /// Materializes a native value into a [`mwl_runtime::Value`] at
    /// `base + offset`.
    fn store_value(
        &mut self,
        base: Value,
        offset: i32,
        value: Value,
        ty: Ty,
    ) -> Result<(), CodegenError> {
        let tag = tag_of(ty)?;
        let bits = match ty {
            // The payload is a `u64`; a `bool` occupies one byte of it, so the
            // other seven are zeroed rather than left as whatever the register
            // happened to hold.
            Ty::Bool => Some(self.b.ins().uextend(types::I64, value)),
            // Stored at its own type: the payload slot is eight bytes either
            // way, so a float needs no bitcast to reach it.
            Ty::Float => Some(value),
            _ => Some(value),
        };
        self.store_tag_and_bits(base, offset, tag, bits)
    }

    fn store_tag_and_bits(
        &mut self,
        base: Value,
        offset: i32,
        tag: Tag,
        bits: Option<Value>,
    ) -> Result<(), CodegenError> {
        let tag_offset =
            offset + i32::try_from(MwlValue::TAG_OFFSET).map_err(|_| internal("a tag past i32"))?;
        let bits_offset = offset
            + i32::try_from(MwlValue::BITS_OFFSET).map_err(|_| internal("a payload past i32"))?;

        let tag_value = self.b.ins().iconst(types::I8, i64::from(tag as u8));
        self.b.ins().store(trusted(), tag_value, base, tag_offset);
        let bits = match bits {
            Some(bits) => bits,
            None => self.b.ins().iconst(types::I64, 0),
        };
        self.b.ins().store(trusted(), bits, base, bits_offset);
        Ok(())
    }

    /// Reads a native value back out of a [`mwl_runtime::Value`] at
    /// `base + offset`.
    ///
    /// The tag is *not* re-checked: every helper already validates its own
    /// arguments' tags and returns `FATAL` on a mismatch — see
    /// `mwl_runtime::helpers`' own docs for why that check lives there — and a
    /// well-typed program cannot produce one here by construction.
    fn load_value(&mut self, base: Value, offset: i32, ty: Ty) -> Result<Value, CodegenError> {
        let bits_offset = offset
            + i32::try_from(MwlValue::BITS_OFFSET).map_err(|_| internal("a payload past i32"))?;
        Ok(match ty {
            Ty::Bool => {
                let wide = self.b.ins().load(types::I64, trusted(), base, bits_offset);
                self.b.ins().ireduce(types::I8, wide)
            }
            Ty::Float => self.b.ins().load(types::F64, trusted(), base, bits_offset),
            Ty::Void => return Err(internal("reading a value of representation `void`")),
            _ => self.b.ins().load(types::I64, trusted(), base, bits_offset),
        })
    }

    // -- imports ------------------------------------------------------------

    fn runtime_ref(
        &mut self,
        symbol: &'static str,
        sig: RuntimeSig,
    ) -> Result<codegen::ir::FuncRef, CodegenError> {
        if let Some(reference) = self.frefs.get(symbol) {
            return Ok(*reference);
        }
        let signature = match sig {
            RuntimeSig::Helper => &self.sigs.helper,
            RuntimeSig::Safepoint => &self.sigs.safepoint,
            RuntimeSig::Probe => &self.sigs.probe,
            RuntimeSig::ProbeCall => &self.sigs.probe_call,
            RuntimeSig::ProbeCallExit => &self.sigs.probe_call_exit,
            RuntimeSig::StrNew => &self.sigs.str_new,
            RuntimeSig::StrConcat => &self.sigs.str_concat,
            RuntimeSig::Refcount => &self.sigs.refcount,
            RuntimeSig::PtrToPtr => &self.sigs.ptr_to_ptr,
            RuntimeSig::Raise => &self.sigs.raise,
            RuntimeSig::InstanceOf => &self.sigs.instanceof,
            RuntimeSig::ArrayNew => &self.sigs.array_new,
            RuntimeSig::ArrayGet => &self.sigs.array_get,
            RuntimeSig::ArraySet => &self.sigs.array_set,
            RuntimeSig::ArrayAppend => &self.sigs.array_append,
            RuntimeSig::ArrayNextSlot => &self.sigs.array_next_slot,
            RuntimeSig::ArrayKeyAt => &self.sigs.array_key_at,
            RuntimeSig::ArrayValueAt => &self.sigs.array_value_at,
        };
        let id = self
            .module
            .declare_function(symbol, Linkage::Import, signature)
            .map_err(|source| CodegenError::Cranelift {
                function: self.f.name.clone(),
                source: Box::new(source),
            })?;
        let reference = self.module.declare_func_in_func(id, self.b.func);
        self.frefs.insert(symbol, reference);
        Ok(reference)
    }
}

/// Which of [`Signatures`]' shapes a runtime symbol has.
#[derive(Clone, Copy)]
enum RuntimeSig {
    Helper,
    Safepoint,
    Probe,
    ProbeCall,
    ProbeCallExit,
    StrNew,
    StrConcat,
    Refcount,
    PtrToPtr,
    Raise,
    InstanceOf,
    ArrayNew,
    ArrayGet,
    ArraySet,
    ArrayAppend,
    ArrayNextSlot,
    ArrayKeyAt,
    ArrayValueAt,
}

/// The symbol name `mwl-runtime` exports for one [`Helper`] tag.
///
/// This mapping is why `mwl-codegen` exists as the crate that depends on both:
/// `mwl-runtime` deliberately does not know `mwl_ir::Helper`, and `mwl-ir`
/// deliberately does not know a symbol name.
fn helper_symbol(helper: Helper) -> Result<&'static str, CodegenError> {
    Ok(match helper {
        Helper::IntToString => "mwl_int_to_string",
        Helper::UintToString => "mwl_uint_to_string",
        Helper::FloatToString => "mwl_float_to_string",
        Helper::BoolToString => "mwl_bool_to_string",
        Helper::IntTruthy => "mwl_int_truthy",
        Helper::UintTruthy => "mwl_uint_truthy",
        Helper::FloatTruthy => "mwl_float_truthy",
        Helper::StrTruthy => "mwl_str_truthy",
        Helper::EchoStr => "mwl_echo_str",
        Helper::ArrayTruthy => "mwl_array_truthy",
        other => {
            return Err(CodegenError::Unsupported(format!(
                "the runtime helper {other:?}"
            )));
        }
    })
}

/// A human-readable name for an instruction this slice does not lower.
fn describe(kind: &InstKind) -> String {
    let what = match kind {
        InstKind::Call { .. } => "a resolved method call",
        InstKind::New { .. } => "`new`",
        InstKind::FieldGet { .. } => "a property read",
        InstKind::FieldSet { .. } => "a property write",
        InstKind::InstanceOf { .. } => "`instanceof`",
        InstKind::Concat { .. } => "`.` string concatenation",
        _ => "this instruction",
    };
    what.to_owned()
}
