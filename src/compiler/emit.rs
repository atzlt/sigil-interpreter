use lasso::Spur;

use crate::{
    compiler::{
        compile::{CompileError, Compiler, Result},
        label::Label,
    },
    emit, emit_args,
    functions::FnLookupKey,
    types::StructDef,
    value::Value,
};

// ── Value constructors ──

impl<'a> Compiler<'a> {
    /// Resolve an identifier to a register. For locals, returns the held register directly.
    /// For globals/upvalues, emits GETGLB/GETUPVAL into a temp register.
    pub(super) fn emit_identifier(&mut self, name: Spur) -> Result<u8> {
        if let Some(reg) = self.try_resolve_local(name) {
            return Ok(reg);
        }
        if let Some(up_idx) = self.resolve_upvalue(name) {
            let reg = self.alloc_temp()?;
            emit!(self.chunk_mut(), GETUPVAL, reg, wide up_idx as u16);
            return Ok(reg);
        }
        let slot = self
            .resolve_global(name)
            .ok_or_else(|| CompileError::UndefinedVariable {
                name: self.intern_resolve(&name).to_string(),
                diag: (self.prev_span().clone(), "undefined variable".to_string()),
            })?;
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), GETGLB, reg, wide slot);
        Ok(reg)
    }

    pub(super) fn emit_number(&mut self, n: f64) -> Result<u8> {
        let k = self.chunk_mut().add_constant(Value::Number(n));
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADK, reg, wide k);
        Ok(reg)
    }

    pub(super) fn emit_string(&mut self, spur: Spur) -> Result<u8> {
        let s = self.intern_resolve(&spur).to_string();
        let k = self.chunk_mut().add_constant(Value::String(s));
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADK, reg, wide k);
        Ok(reg)
    }

    pub(super) fn emit_bool(&mut self, b: bool) -> Result<u8> {
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADBOOL, reg, b as u8);
        Ok(reg)
    }

    pub(super) fn emit_nil(&mut self) -> Result<u8> {
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADNIL, reg);
        Ok(reg)
    }

    // ── Function calls ──

    pub(super) fn emit_lang_item_call(
        &mut self,
        fun: FnLookupKey,
        args: &[u8],
        target: Option<u8>,
    ) -> Result<u8> {
        let fn_id = self
            .resolve_fn(&fun)
            .ok_or_else(|| CompileError::UndefinedFunction {
                name: fun.to_string(),
                diag: (
                    self.current_span().clone(),
                    "this language item is not defined".to_string(),
                ),
            })?;
        let reg = self.target_or_reuse(target, args)?;
        self.emit_callk(reg, fn_id, self.reg_watermark(), args);
        self.free_other_temps(reg, args);
        Ok(reg)
    }

    pub(super) fn emit_callk(&mut self, dst: u8, fn_slot: usize, frame_offset: u8, args: &[u8]) {
        let argc = args.len();
        emit!(self.chunk_mut(), CALLK, dst, wide fn_slot as u16, frame_offset, argc);
        self.chunk_mut().append(args);
    }

    pub(super) fn emit_call(&mut self, dst: u8, reg: u8, frame_offset: u8, args: &[u8]) {
        let argc = args.len();
        emit!(self.chunk_mut(), CALL, dst, reg, frame_offset, argc);
        self.chunk_mut().append(args);
    }

    // ── Closures ──

    /// Emit a `CLOSURE` instruction for a named nested function.
    pub(super) fn emit_closure(
        &mut self,
        name: Spur,
        proto_idx: u16,
        upvalues: &[crate::compiler::compile::UpvalueDescriptor],
    ) -> Result<()> {
        let reg = self.alloc_held()?;
        emit!(self.chunk_mut(), CLOSURE, reg, wide proto_idx);
        for uv in upvalues {
            self.chunk_mut().emit(uv.is_local as u8);
            self.chunk_mut().emit(uv.index);
        }
        self.add_local(name, reg);
        Ok(())
    }

    /// Emit a `CLOSURE` for an anonymous closure expression.
    /// Returns a temp register holding the resulting `Value::Closure`.
    pub(super) fn emit_closure_temp(
        &mut self,
        proto_idx: u16,
        upvalues: &[crate::compiler::compile::UpvalueDescriptor],
    ) -> Result<u8> {
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), CLOSURE, reg, wide proto_idx);
        for uv in upvalues {
            self.chunk_mut().emit(uv.is_local as u8);
            self.chunk_mut().emit(uv.index);
        }
        Ok(reg)
    }

    // ── Structs ──

    /// Emit a NEWSTRUCT instruction.
    pub(super) fn emit_newstruct(&mut self, dst: u8, def_id: u16, def: &StructDef, regs: &[u8]) {
        emit!(self.chunk_mut(), NEWSTRUCT, dst, wide def_id, regs.len());
        for (i, &reg) in regs.iter().enumerate() {
            let name_str = self.intern_resolve(&def.fields[i].0).to_string();
            let name_k = self.chunk_mut().add_constant(Value::String(name_str));
            self.chunk_mut().emit_wide(name_k);
            self.chunk_mut().emit(reg);
        }
    }

    /// Emit GETFIELD: read `obj.field_name` into a new temp register.
    /// Returns the destination register.
    pub(super) fn emit_getfield(&mut self, obj_reg: u8, field_name: Spur) -> Result<u8> {
        let name_str = self.intern_resolve(&field_name).to_string();
        let name_k = self.chunk_mut().add_constant(Value::String(name_str));
        let dst = self.alloc_temp()?;
        emit!(self.chunk_mut(), GETFIELD, dst, obj_reg, wide name_k);
        Ok(dst)
    }

    /// Emit SETFIELD: write `val_reg` into `obj_reg.field_name`.
    pub(super) fn emit_setfield(&mut self, obj_reg: u8, field_name: Spur, val_reg: u8) {
        let name_str = self.intern_resolve(&field_name).to_string();
        let name_k = self.chunk_mut().add_constant(Value::String(name_str));
        emit!(self.chunk_mut(), SETFIELD, obj_reg, wide name_k, val_reg);
    }

    // ── Field chains (nested field access / assignment) ──

    /// Emit a chain of GETFIELD operations for a read expression like `v.x.re`.
    /// `base_name` is the root variable, `fields` is the chain of field names.
    /// Returns the final register holding the innermost field value.
    pub(super) fn emit_field_chain_get(&mut self, base_name: Spur, fields: &[Spur]) -> Result<u8> {
        let mut cur_reg = self.emit_identifier(base_name)?;
        let prev_temp = cur_reg; // track if we need to free the previous temp

        for (i, &field) in fields.iter().enumerate() {
            let next_reg = self.emit_getfield(cur_reg, field)?;
            // Free the previous temp if it wasn't a held register from emit_identifier.
            // emit_identifier returns held regs for locals; free_temp is a no-op on those.
            if i > 0 {
                self.frame_mut().regs.free_temp(cur_reg);
            }
            cur_reg = next_reg;
        }
        // Free the base register if it was a temp (global/upvalue).
        self.frame_mut().regs.free_temp(prev_temp);

        Ok(cur_reg)
    }

    /// Emit a chain ending in SETFIELD for assignment like `v.x.re = value`.
    /// `base_name` is the root variable, `fields` is the chain; the last field
    /// is the SETFIELD target, all preceding fields emit GETFIELD to traverse.
    pub(super) fn emit_field_chain_assign(
        &mut self,
        base_name: Spur,
        fields: &[Spur],
    ) -> Result<()> {
        assert!(
            !fields.is_empty(),
            "field chain must have at least one field"
        );

        let base_reg = self.emit_identifier(base_name)?;

        // Traverse intermediate fields with GETFIELD.
        let mut cur_reg = base_reg;
        for &field in &fields[..fields.len() - 1] {
            let next_reg = self.emit_getfield(cur_reg, field)?;
            // Free the intermediate temp (not the base, which may be held).
            self.frame_mut().regs.free_temp(cur_reg);
            cur_reg = next_reg;
        }

        // Parse the RHS value.
        let val_reg = self.expression(None)?;

        // Emit SETFIELD for the last field.
        let last_field = fields[fields.len() - 1];
        self.emit_setfield(cur_reg, last_field, val_reg);

        self.frame_mut().regs.free_temp(val_reg);
        self.frame_mut().regs.free_temp(cur_reg);

        Ok(())
    }

    // ── Misc ──

    pub(super) fn emit_move(&mut self, dst: u8, src: u8) {
        if dst != src {
            emit!(self.chunk_mut(), MOVE, dst, src);
        }
    }

    pub(super) fn emit_safety_net(&mut self) -> Result<()> {
        let nil_reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADNIL, nil_reg);
        emit!(self.chunk_mut(), RETURN, nil_reg);
        Ok(())
    }

    pub(super) fn target_or_reuse(&mut self, target: Option<u8>, sources: &[u8]) -> Result<u8> {
        if let Some(t) = target {
            Ok(t)
        } else {
            self.reuse_or_alloc(sources)
        }
    }

    // ── Labels ──

    pub(super) fn new_label(&mut self) -> Label {
        self.frame_mut().labels.alloc()
    }

    pub(super) fn emit_forward_test(&mut self, reg: u8) -> Label {
        let label = self.frame_mut().labels.alloc();
        let ip = self.chunk().end();
        emit!(self.chunk_mut(), TEST, reg, wide 0);
        self.frame_mut()
            .labels
            .add_ref(label, ip, crate::compiler::label::RefKind::Test);
        label
    }

    pub(super) fn emit_forward_jmp(&mut self) -> Label {
        let label = self.frame_mut().labels.alloc();
        let ip = self.chunk().end();
        emit!(self.chunk_mut(), JMP, wide 0);
        self.frame_mut()
            .labels
            .add_ref(label, ip, crate::compiler::label::RefKind::Jmp);
        label
    }

    pub(super) fn emit_forward_jmp_to(&mut self, label: Label) {
        let ip = self.chunk().end();
        emit!(self.chunk_mut(), JMP, wide 0);
        self.frame_mut()
            .labels
            .add_ref(label, ip, crate::compiler::label::RefKind::Jmp);
    }

    pub(super) fn emit_here_label(&mut self) -> Label {
        let label = self.frame_mut().labels.alloc();
        let ip = self.chunk().end();
        let chunk_idx = self.frame().chunk_idx;
        let labels = &mut self.frames.last_mut().unwrap().labels;
        labels.resolve(label, ip, &mut self.chunks[chunk_idx]);
        label
    }

    pub(super) fn emit_jmp_to(&mut self, label: Label) {
        let target = self.frame().labels.ip_of(label);
        let ip = self.chunk().end();
        let offset = target as isize - ip as isize;
        let bytes = (offset as i16).to_le_bytes();
        emit!(self.chunk_mut(), JMP, bytes[0], bytes[1]);
    }

    pub(super) fn place_label(&mut self, label: Label) {
        let target_ip = self.chunk().end();
        let chunk_idx = self.frame().chunk_idx;
        let labels = &mut self.frames.last_mut().unwrap().labels;
        labels.resolve(label, target_ip, &mut self.chunks[chunk_idx]);
    }
}
