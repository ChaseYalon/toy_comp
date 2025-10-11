use crate::{Lexer, Parser, Compiler};

macro_rules! compile_code {
    ($o:ident, $i:expr) => {
        let mut l = Lexer::new();
        let mut p = Parser::new();
        let mut c = Compiler::new();
        let $o = c.compile(p.parse(l.lex($i.to_string())));
    };
}

#[test]
fn test_compiler_int_lit() {
    compile_code!(code_fn, "6");
    assert_eq!(6, code_fn());
}

#[test]
fn test_compiler_int_multi_char_lit() {
    compile_code!(code_fn, "16");
    assert_eq!(16, code_fn());
}

#[test]
fn test_compiler_int_infix_1() {
    compile_code!(code_fn, "18 - 3");
    assert_eq!(15, code_fn());
}

#[test]
fn test_compiler_int_infix_2() {
    compile_code!(code_fn, "24 / 6 - 3");
    assert_eq!(1, code_fn());
}
