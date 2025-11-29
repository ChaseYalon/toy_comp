use crate::codegen::tir::AstToIrConverter;

mod tir;

pub struct Generator {
    ast_t_ir: AstToIrConverter,
}
