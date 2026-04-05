// SPDX-FileCopyrightText: © 2026 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE
use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use encoding_rs::Encoding;
use encoding_rs::UTF_8;
use encoding_rs_io::DecodeReaderBytesBuilder;

use metrowrap::NamedString;
use metrowrap::SourceType;
use metrowrap::assembler;
use metrowrap::compiler;
use metrowrap::preprocessor;

#[derive(Parser, Debug)]
#[command(author, version, about = "MWCC bridge for assembly injection")]
#[command(allow_external_subcommands = true)]
#[command(override_usage = "mw [OPTIONS]… -o <output> [COMPILER_FLAGS]… <file>")]
#[command(
    name = "mw",
    help_template = "\
{before-help}{name} {version}
{author-with-newline}{about-with-newline}
Usage: {usage}

Arguments:
  [OPTIONS]...         {name} options (described below)
  [COMPILER_FLAGS]...  Flags passed directly to the compiler
  <infile>             The input file to process (or '-' for stdin)

Options:
{options}
"
)]
struct Args {
    #[arg(help = "Output object file", short)]
    output: PathBuf,

    // #[arg(help = "Input C file", required = false, default_value = "-")]
    // c_file: PathBuf,
    #[arg(long, default_value = "mwccpsp.exe")]
    mwcc_path: PathBuf,

    #[arg(long, default_value = "mipsel-linux-gnu-as")]
    as_path: String,

    #[arg(long, default_value = "allegrex")]
    as_march: String,

    #[arg(long, default_value = "32")]
    as_mabi: String,

    #[arg(long)]
    use_wibo: bool,

    #[arg(long, default_value = "wibo")]
    wibo_path: PathBuf,

    #[arg(long)]
    asm_dir: Option<PathBuf>,

    #[arg(long)]
    macro_inc_path: Option<PathBuf>,

    #[arg(long)]
    src_dir: Option<PathBuf>,

    #[arg(long)]
    target_encoding: Option<String>,
    
    /// Split monolithic sections into individual per-symbol sections.
    #[arg(long)]
    split_sections: bool,

    /// Use plain section names (.text, .rodata) instead of .text.<name>.
    /// Each symbol still gets its own section, but all share the same name.
    #[arg(long)]
    split_plain_names: bool,

    /// Override the ELF e_flags field in the output object.
    #[arg(long)]
    elf_flags: Option<String>,

    /// This catches everything else: unknown flags AND the file path.
    /// trailing_var_arg means everything after the first "unknown"
    /// or positional is dumped here.
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    rest: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = Args::parse();

    let encoding = if let Some(target_encoding) = args.target_encoding {
        Encoding::for_label(target_encoding.as_bytes()).expect("encoding")
    } else {
        UTF_8
    };

    let Some(possible_infile) = args.rest.last() else {
        eprintln!("missing input file");
        std::process::exit(1);
    };

    let infile = if possible_infile == "-" || PathBuf::from(possible_infile).is_file() {
        possible_infile.clone()
    } else {
        eprintln!("cannot find input file: {possible_infile}");
        std::process::exit(1);
    };

    args.rest.pop();

    // TODO: cflags vec!["-Itests/data".to_string()], // Flags would be parsed from extra args
    let compiler =
        compiler::Compiler::new(args.rest, args.mwcc_path, args.use_wibo, args.wibo_path);

    let assembler = assembler::Assembler {
        as_path: args.as_path,
        as_march: args.as_march,
        as_mabi: args.as_mabi,
        as_flags: vec!["-G0".to_string()],
        macro_inc_path: args.macro_inc_path,
    };

    let preprocessor = Arc::new(preprocessor::Preprocessor {
        asm_dir_prefix: args.asm_dir,
    });

    // In GCC, the file is usually the last positional argument.

    let c_reader = if infile == "-" {
        let mut reader = DecodeReaderBytesBuilder::new()
            // .encoding(Some(encoding))
            .build(io::stdin().lock());
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        NamedString {
            source: SourceType::StdIn,
            content,
            encoding,
            src_dir: args.src_dir.unwrap_or(PathBuf::from(".")),
        }
    } else {
        let mut reader = DecodeReaderBytesBuilder::new()
            // .encoding(Some(encoding))
            .bom_sniffing(true)
            .build(File::open(&infile)?);
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        NamedString {
            source: SourceType::Path(infile.clone()),
            content,
            encoding,
            src_dir: PathBuf::from(infile)
                .parent()
                .unwrap_or(&PathBuf::from("."))
                .to_path_buf(),
        }
    };

    let elf_flags = args.elf_flags.map(|s| {
        if s.starts_with("0x") || s.starts_with("0X") {
            u32::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16)
        } else {
            s.parse::<u32>()
        }
        .expect("invalid value for --elf-flags")
    });

    if let Err(e) = metrowrap::process_c_file(
        &c_reader,
        &args.output,
        &preprocessor,
        &compiler,
        &assembler,
        args.split_sections,
        args.split_plain_names,
        elf_flags,
    ) {
        eprintln!("failed to process c file: {:?}", e);
        std::process::exit(1);
    }


    Ok(())
}
