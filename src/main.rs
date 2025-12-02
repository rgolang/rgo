use compiler::compile;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let input = args.next();
    let output = args.next();
    if args.next().is_some() {
        return Err("expected at most two arguments: <input> <output>".into());
    }

    match (input, output) {
        (None, None) => {
            let stdin = stdin();
            let stdout = stdout();
            let mut writer = BufWriter::new(stdout.lock());
            compile(BufReader::new(stdin.lock()), &mut writer)?;
        }
        (Some(input_path), None) => {
            let file = File::open(&input_path)?;
            let stdout = stdout();
            let mut writer = BufWriter::new(stdout.lock());
            compile(BufReader::new(file), &mut writer)?;
        }
        (Some(input_path), Some(output_path)) => {
            let input = File::open(input_path)?;
            let mut output = BufWriter::new(File::create(output_path)?);
            compile(BufReader::new(input), &mut output)?;
        }
        (None, Some(_)) => {
            return Err("an output path requires an input path".into());
        }
    }

    Ok(())
}
