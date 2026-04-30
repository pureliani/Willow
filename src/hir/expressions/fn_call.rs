use crate::{
    ast::{expr::Expr, Span},
    hir::{
        builders::{Builder, InBlock},
        instructions::InstrId,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_fn_call_expr(
        &mut self,
        left: Expr,
        args: Vec<Expr>,
        span: Span,
    ) -> InstrId {
        let func_id = self.build_expr(left);

        let mut arg_ids = Vec::with_capacity(args.len());
        for arg in args {
            arg_ids.push(self.build_expr(arg));
        }

        self.emit_call(func_id, arg_ids, span)
    }
}
