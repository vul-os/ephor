//! The `pier` binary: parse, marshal an [`Invocation`], dispatch, exit with its code.
//!
//! Deliberately thin. Everything decidable lives in the library so it is reachable from tests —
//! in particular [`pier_cli::run::run`] writes to a `dyn Write`, so the "no command ever reports
//! success it did not achieve" test can read what every row prints.

use std::io::Write;
use std::path::PathBuf;

use pier_cli::certs::CertRequest;
use pier_cli::cli;
use pier_cli::run::{run, Invocation};

fn main() {
    let matches = cli::build().get_matches();
    let Some((key, leaf)) = cli::selected(&matches) else {
        // clap's `subcommand_required` + `arg_required_else_help` make this unreachable in
        // practice; refusing beats guessing.
        eprintln!("REFUSED: no subcommand");
        std::process::exit(pier_cli::run::exit::USAGE);
    };

    let get = |m: &clap::ArgMatches, k: &str| m.get_one::<String>(k).cloned();
    let cert = if key == "certs add" {
        Some(CertRequest {
            domain: get(leaf, "domain").unwrap_or_default(),
            ingress: get(leaf, "ingress").unwrap_or_default(),
            acme_delegation: get(leaf, "acme-delegation"),
            ca: get(leaf, "ca").unwrap_or_else(|| "letsencrypt.org".to_string()),
            acme_account: get(leaf, "acme-account"),
            exclude_tls_alpn: leaf.get_flag("exclude-tls-alpn"),
            caa_enforcement_established: leaf.get_flag("caa-enforcement-established"),
        })
    } else {
        None
    };

    let inv = Invocation {
        id: if key == "certs add" {
            get(leaf, "box-id")
        } else {
            get(leaf, "id")
        },
        service: get(leaf, "service"),
        token: get(&matches, "token")
            .or_else(|| get(leaf, "token"))
            .map(PathBuf::from),
        coordinator: get(&matches, "coordinator").or_else(|| get(leaf, "coordinator")),
        cert,
        key,
    };

    let mut out = std::io::stdout().lock();
    let code = run(&inv, &mut out);
    let _ = out.flush();
    std::process::exit(code);
}
