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

pub fn last_slug(path: &str) -> &str {
    if let Some(slash_pos) = path.rfind('/') {
        &path[slash_pos + 1..]
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::compile;
    use std::io::Cursor;

    #[test]
    fn compile_simple_program() {
        let source = r#"
int: @/int
str: @/str
add: @/add
exit: @/exit
write: @/write
sprintf: @/sprintf

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
