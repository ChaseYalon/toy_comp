use crate::codegen::tir::AstToIrConverter;

mod tir;

pub struct Generator {
    ast_t_ir: AstToIrConverter,
}
impl Generator {
    pub fn new() -> Generator {
        return Generator {
            ast_t_ir: AstToIrConverter::new(),
        };
    }
}
