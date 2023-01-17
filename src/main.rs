use std::{
    io::{stdin, stdout, BufRead, ErrorKind, Write},
    process::exit,
};

use crate::pipeline::Pipeline;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use regex::Regex;

mod pipeline;

static USAGE: &str = r#"Usage: rangler [commands]
    filter <regex> // excludes lines that do not match",
    append <quoted string> // appends the text in quotes to every line
    prepend <quoted string> // prepends the text in quotes to every line
    trim // removes whitespace at both ends of every line
    lower // converts English letters to lower case
    upper // converts English letters to upper case
    dedupe // dedupes lines"#;

fn main() {
    match inner_main() {
        Ok(()) => exit(0),
        Err(message) => {
            println!("{}", message);
            println!("{}", USAGE);
            exit(0)
        }
    }
}

fn inner_main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let mut pipeline = Pipeline::build_pipeline(&args[1..])?;

    let mut total_bytes_read = 0;
    let mut bytes_at_last_message = 0;

    let progress = ProgressBar::new_spinner()
        .with_style(ProgressStyle::with_template("[{elapsed_precise}] {msg}").unwrap());

    let mut std_out = std::io::BufWriter::with_capacity(1_000_000, stdout());
    let mut std_in = std::io::BufReader::with_capacity(1_000_000, stdin());
    let mut line_of_bytes: Vec<u8> = Vec::new();

    loop {
        let mut buffer = std_in.fill_buf().map_err(|e| "IO Error")?;

        if buffer.len() == 0 {
            break;
        }

        loop {
            let consumed = buffer
                .read_until(b'\n', &mut line_of_bytes)
                .map_err(|e| "IO Error")?;

            if consumed == 0 || line_of_bytes.last().unwrap() == &b'\n' {
                break;
            }
        }

        match std::str::from_utf8(&line_of_bytes) {
            Ok(line_of_text) => {
                let transforemd_line = pipeline.apply(line_of_text);

                if let Some(line) = transforemd_line {
                    std_out
                        .write_all((line + "\n").as_bytes())
                        .expect("IO Error");
                }
            }
            Err(_) => { /* todo ignore */ }
        };

        total_bytes_read += line_of_bytes.len();

        if total_bytes_read > bytes_at_last_message + 256_000 {
            progress.inc(line_of_bytes.len() as u64);

            let message = format!(
                "{} read, {} stored",
                HumanBytes(total_bytes_read as u64),
                HumanBytes(pipeline.get_memory() as u64)
            );
            progress.set_message(message);
            bytes_at_last_message = total_bytes_read;

            std_out.flush().expect("IO Error");
        }

        std_in.consume(line_of_bytes.len());
        line_of_bytes.clear();
    }

    std_out.flush().expect("IO Error");
    progress.finish();
    Ok(())
}
