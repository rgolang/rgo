pub mod compiler;
pub mod debug_tools;

pub use compiler::compile;
pub use compiler::error::{Code, Error};

pub fn escape_literal_for_rodata(literal: &str) -> String {
    fn append_part(output: &mut String, part: &str) {
        if !output.is_empty() {
            output.push_str(", ");
        }
        output.push_str(part);
    }

    fn flush_chunk(output: &mut String, chunk: &mut Vec<u8>) {
        if chunk.is_empty() {
            return;
        }
        let mut literal = String::from("\"");
        for &byte in chunk.iter() {
            match byte {
                b'"' => literal.push_str("\\\""),
                other => literal.push(other as char),
            }
        }
        literal.push('"');
        append_part(output, &literal);
        chunk.clear();
    }

    let mut output = String::new();
    let mut chunk = Vec::new();
    for &byte in literal.as_bytes() {
        match byte {
            b'\n' => {
                flush_chunk(&mut output, &mut chunk);
                append_part(&mut output, "10");
            }
            b'\r' => {
                flush_chunk(&mut output, &mut chunk);
                append_part(&mut output, "13");
            }
            b'\t' => {
                flush_chunk(&mut output, &mut chunk);
                append_part(&mut output, "9");
            }
            b if b == b'\\' || b == b'"' || b == b' ' || (0x21..=0x7e).contains(&b) => {
                chunk.push(byte);
            }
            other => {
                flush_chunk(&mut output, &mut chunk);
                append_part(&mut output, &format!("0x{other:02x}"));
            }
        }
    }

    flush_chunk(&mut output, &mut chunk);

    if output.is_empty() {
        return "\"\"".to_string();
    }
    output
}

pub fn sanitize_function_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::compile;
    use std::io::Cursor;

    #[test]
    fn compile_simple_program() {
        let source = r#"
int: @int
str: @str
add: @add
exit: @exit
write: @write
sprintf: @sprintf

print_int: (value: int) {
    sprintf("%d", value, (res: str){
        write(res, exit(0))    
    })
}

add_five: (ok:(int)) {
    add(5, 0, ok)
}

main: () {
    add_five((res: int) {
        print_int(res)
    })
}
        "#;
        let mut output = Vec::new();
        compile(Cursor::new(source.as_bytes()), "main", &mut output)
            .expect("compiler produced asm");
        let asm = String::from_utf8(output).expect("valid utf8");
        assert!(asm.contains("global _start"));
        assert!(asm.contains("global add_five"));
    }

    #[test]
    fn compile_direct_builtin_references() {
        let source = r#"
main: () {
    @add(5, 0, (res: @int) {
        (s: @str) = @sprintf("%d", res)
        @write(s, @exit(0))
    })
}
        "#;
        let mut output = Vec::new();
        compile(Cursor::new(source.as_bytes()), "main", &mut output)
            .expect("compiler produced asm");
        let asm = String::from_utf8(output).expect("valid utf8");
        assert!(asm.contains("global _start"));
        assert!(asm.contains("extern sprintf"));
        assert!(asm.contains("extern write"));
    }

    #[test]
    fn compile_user_defined_puts() {
        let source = r#"
str: @str
exit: @exit
write: @write

puts: (s: str, ok:()) {
    write(s, () {
        write("\n", ok)
    })
}

main: () {
    puts("hello", exit(0))
}
        "#;
        let mut output = Vec::new();
        compile(Cursor::new(source.as_bytes()), "main", &mut output)
            .expect("compiler produced asm");
        let asm = String::from_utf8(output).expect("valid utf8");
        assert!(asm.contains("global puts"));
        assert!(asm.contains("extern write"));
        assert!(asm.contains("extern exit"));
        assert!(!asm.contains("extern puts"));
    }

    #[test]
    fn reject_builtin_puts_import() {
        let source = r#"
puts: @puts
main: () {
    puts("hello", exit(0))
}
        "#;
        let mut output = Vec::new();
        let err = compile(Cursor::new(source.as_bytes()), "main", &mut output)
            .expect_err("puts should no longer be a builtin import");
        assert!(err.to_string().contains("@puts"));
    }
}
