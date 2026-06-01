use compiler::compile;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let input = args.next();
    let target = args.next();
    let output = args.next();
    if args.next().is_some() {
        return Err("expected exactly three arguments: <input> <target> <output>".into());
    }

    let (input_path, target, output_path) = match (input, target, output) {
        (Some(input), Some(target), Some(output)) => (input, target, output),
        _ => return Err("compiler requires <input> <target> <output>".into()),
    };

    let input = File::open(input_path)?;
    let mut output = BufWriter::new(File::create(output_path)?);
    compile(BufReader::new(input), &target, &mut output)?;

    Ok(())
}
