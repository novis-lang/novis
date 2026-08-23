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
use cranelift_module::{DataDescription, Linkage, Module};
use mwl_ir::Ty;
use mwl_ir::ids::{BlockId, ValueId};
use mwl_ir::ir::{BasicBlock, BinOp, Function, Helper, Inst, InstKind, Terminator, UnOp};
use mwl_runtime::{DEBUG_FLAGS_OFFSET, OK, SAFEPOINT_OFFSET, Tag, Value as MwlValue};
use rustc_hash::FxHashMap;

use crate::ty::{clif_ty, tag_of};
use crate::{CodegenError, Signatures};

/// Size of one 16-byte [`mwl_runtime::Value`], as an offset multiplier.
const VALUE_SIZE: i32 = 16;

/// `align_shift` for a 16-byte-aligned stack slot: 2^4 == 16.
const VALUE_ALIGN_SHIFT: u8 = 4;

/// Memory flags for a load or store this frame fully controls — a stack slot
/// it just allocated, or the `out` pointer its caller promised is writable.
fn trusted() -> MemFlags {
    MemFlags::trusted()
}

/// Memory flags for reading [`mwl_runtime::Ctx`]'s two hot words.
///
/// Deliberately *not* [`MemFlags::trusted`]: `trusted` asserts nothing about
/// aliasing today, but the safepoint word is written from outside the running
/// frame (a CPU-limit watchdog, a cancellation), so the load must stay a real
/// load rather than becoming something Cranelift may prove redundant. Marking
/// it `readonly` — which is what a future "this is immutable" annotation would
/// mean — would be exactly wrong.
fn ctx_word() -> MemFlags {
    MemFlags::new().with_notrap()
}

/// Emits `f` into `ctx.func`, which the caller has already given the ABI
/// signature.
pub(crate) fn emit_function(
    module: &mut JITModule,
    ctx: &mut codegen::Context,
    fn_ctx: &mut FunctionBuilderContext,
    sigs: &Signatures,
    literals: &mut usize,
    f: &Function,
) -> Result<(), CodegenError> {
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
        literals,
        f,
        values: FxHashMap::default(),
        blocks,
        phi_counts,
        frefs: FxHashMap::default(),
        ctx_p,
        args_p,
        out_p,
    };
    emitter.emit_blocks()?;
    emitter.b.seal_all_blocks();
    emitter.b.finalize();
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

fn internal(what: &str) -> CodegenError {
    CodegenError::Unsupported(format!("{what} (this is a bug in mwl-ir or mwl-codegen)"))
}

struct Emitter<'a, 'f> {
    b: FunctionBuilder<'f>,
    module: &'a mut JITModule,
    sigs: &'a Signatures,
    literals: &'a mut usize,
    f: &'a Function,
    /// Every SSA value defined so far, with the representation it was defined
    /// at — the IR carries that on the defining instruction, and an operand
    /// needs it back to pick an instruction (`sdiv` vs `udiv` vs `fdiv`).
    values: FxHashMap<u32, (Value, Ty)>,
    blocks: FxHashMap<u32, Block>,
    phi_counts: FxHashMap<u32, usize>,
    frefs: FxHashMap<&'static str, codegen::ir::FuncRef>,
    ctx_p: Value,
    args_p: Value,
    out_p: Value,
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
                return self.emit_helper(cur, inst, *helper, args);
            }
            InstKind::Retain { operand } => {
                let (value, ty) = self.value(*operand)?;
                self.emit_refcount("mwl_str_retain", value, ty)?;
            }
            InstKind::Release { operand } => {
                let (value, ty) = self.value(*operand)?;
                self.emit_refcount("mwl_str_release", value, ty)?;
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
        let name = format!("mwl_str_{}", *self.literals);
        *self.literals += 1;

        let mut desc = DataDescription::new();
        // A zero-length literal still needs a real address to hand to
        // `mwl_str_new`, which ignores the pointer when the length is zero but
        // is handed one regardless; a one-byte object is the cheapest way to
        // keep the two cases identical here.
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
            i64::try_from(bytes.len()).map_err(|_| internal("a string literal past i64 bytes"))?,
        );
        let callee = self.runtime_ref("mwl_str_new", RuntimeSig::StrNew)?;
        let call = self.b.ins().call(callee, &[address, len]);
        let value = self.b.inst_results(call)[0];
        Ok((value, cur))
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
        helper: Helper,
        args: &[ValueId],
    ) -> Result<Block, CodegenError> {
        let symbol = helper_symbol(helper)?;

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
        let cont = self.emit_status_check(status)?;

        if let Some(ty) = inst.ty {
            let value = self.load_value(out_p, 0, ty)?;
            self.define(inst, value)?;
        }
        let _ = cur;
        Ok(cont)
    }

    /// A retain or release of one refcounted value.
    ///
    /// [`Ty::Str`] and [`Ty::Bytes`] share the `StrHeader` representation, so
    /// they share the primitive. [`Ty::Array`] is [`mwl_ir::Ty::is_refcounted`]
    /// too, but has no runtime representation at all yet, so its refcount
    /// operation has nothing to call.
    fn emit_refcount(
        &mut self,
        symbol: &'static str,
        value: Value,
        ty: Ty,
    ) -> Result<(), CodegenError> {
        match ty {
            Ty::Str | Ty::Bytes => {
                let callee = self.runtime_ref(symbol, RuntimeSig::Refcount)?;
                self.b.ins().call(callee, &[value]);
                Ok(())
            }
            other => Err(CodegenError::Unsupported(format!(
                "a refcount operation on representation {other:?}"
            ))),
        }
    }

    /// ADR 0002's compare-and-branch: on a non-`OK` status, return it onward
    /// unchanged; otherwise carry on in a fresh block.
    ///
    /// The error path does **not** release the frame's live refcounted locals
    /// — see the crate docs' known gap 3 for why that lands with the IR's own
    /// error edge rather than being guessed at here.
    fn emit_status_check(&mut self, status: Value) -> Result<Block, CodegenError> {
        let failed = self
            .b
            .ins()
            .icmp_imm(IntCC::NotEqual, status, i64::from(OK));
        let fail = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(failed, fail, &[], cont, &[]);

        self.b.switch_to_block(fail);
        self.b.ins().return_(&[status]);

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
            other => {
                return Err(CodegenError::Unsupported(format!(
                    "the terminator {other:?}"
                )));
            }
        }
        let _ = cur;
        Ok(())
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
            RuntimeSig::StrNew => &self.sigs.str_new,
            RuntimeSig::Refcount => &self.sigs.refcount,
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
    StrNew,
    Refcount,
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
        // `mwl-runtime`'s known gap 1: `Tag::Array` has no representation, so
        // there is no entry point to call.
        Helper::ArrayTruthy => {
            return Err(CodegenError::Unsupported(
                "`array` truthiness, which has no runtime entry point until \
                 arrays have a representation"
                    .to_owned(),
            ));
        }
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
        InstKind::Concat { .. } => "`.` string concatenation",
        InstKind::ArrayNew { .. } => "an array literal",
        InstKind::ArrayGet { .. } => "an array read",
        InstKind::ArraySet { .. } => "an array write",
        InstKind::ArrayAppend { .. } => "an array append",
        _ => "this instruction",
    };
    what.to_owned()
}
