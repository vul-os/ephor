//! The `pier` binary: parse, dispatch, exit with the code.
//!
//! Deliberately almost empty. Everything decidable lives in the library so it is reachable from
//! tests — including the clap-to-[`Invocation`](pier_cli::run::Invocation) marshalling, which used
//! to live here and was the one part of this crate no test could see. It panicked on every command
//! while 39 tests stayed green. It is [`pier_cli::cli::invocation`] now, and
//! `tests/end_to_end.rs` drives every row of the table through it.

use std::io::Write;

use pier_cli::cli;
use pier_cli::run::run;

fn main() {
    let matches = cli::build().get_matches();
    let Some(inv) = cli::invocation(&matches) else {
        // clap's `subcommand_required` + `arg_required_else_help` make this unreachable in
        // practice; refusing beats guessing.
        eprintln!("REFUSED: no subcommand");
        std::process::exit(pier_cli::run::exit::USAGE);
    };

    let mut out = std::io::stdout().lock();
    let code = run(&inv, &mut out);
    let _ = out.flush();
    std::process::exit(code);
}
