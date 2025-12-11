pub mod compiler;

pub use compiler::compile;
pub use compiler::error::{CompileError, CompileErrorCode};

#[cfg(test)]
mod tests {
    use super::compile;
    use std::io::Cursor;

    #[test]
    fn compile_simple_program() {
        let source = r#"
@/int
@/add
@/exit
@/write
@/sprintf

print_int: (value: int) {
    sprintf("%d", value, (res: str){
        write(res, exit(0))    
    })
}

add_five: (ok:(int)) {
    add(5, 0, ok)
}

add_five((res: int) {
    print_int(res)
})
        "#;
        let mut output = Vec::new();
        compile(Cursor::new(source.as_bytes()), &mut output).expect("compiler produced asm");
        let asm = String::from_utf8(output).expect("valid utf8");
        assert!(asm.contains("global _start"));
        assert!(asm.contains("global add_five"));
    }
}
