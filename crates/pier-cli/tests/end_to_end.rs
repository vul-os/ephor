//! **Every command, through the real argv path.** Parse → marshal → dispatch, for all 37 rows.
//!
//! # Why this file exists
//!
//! It exists because of a defect nothing else here could see. The clap-to-`Invocation`
//! marshalling used to live in `main.rs`, where a `#[test]` cannot reach it, and it called
//! `ArgMatches::get_one("service")` unconditionally. That method **panics** on an id the matched
//! subcommand never declared, and `--service` exists only on `logs` — so the binary aborted with
//! exit 101 on *literally every command*, while 39 unit tests were green. The unit tests all
//! constructed an `Invocation` by hand and called `run()` directly, jumping over the only code
//! that was broken.
//!
//! The structural fix was to move the marshalling into the library (`cli::invocation`). This file
//! is the behavioural one: it walks the command table, builds a plausible argv for each row, and
//! drives it end to end. A row that panics, or that exits 0 without having done anything, fails
//! here.

use pier_cli::cli;
use pier_cli::run::{exit, run};
use pier_cli::table::{Status, TABLE};

/// A plausible argv for one table row: the path, plus whatever clap marks required.
fn argv(key: &str) -> Vec<String> {
    let mut v: Vec<String> = std::iter::once("pier".to_string())
        .chain(key.split(' ').map(str::to_string))
        .collect();
    if key == "certs add" {
        v.extend(
            ["app.example.com", "--ingress", "ingress.reach.example"]
                .iter()
                .map(|s| s.to_string()),
        );
    }
    v
}

#[test]
fn every_command_survives_the_real_argv_path() {
    let mut ran = 0usize;
    for cmd in TABLE {
        let key = cmd.key();
        let matches = cli::build()
            .try_get_matches_from(argv(&key))
            .unwrap_or_else(|e| panic!("`{key}` does not parse: {e}"));
        let inv = cli::invocation(&matches)
            .unwrap_or_else(|| panic!("`{key}` parsed but produced no Invocation"));
        assert_eq!(inv.key, key, "argv for `{key}` selected the wrong row");

        // The call that used to abort the process.
        let mut buf: Vec<u8> = Vec::new();
        let code = run(&inv, &mut buf);
        let text = String::from_utf8(buf).unwrap();

        assert_ne!(
            code,
            exit::OK,
            "`{key}` exited 0 having done nothing:\n{text}"
        );
        assert!(
            text.contains("NOT IMPLEMENTED") || text.contains("REFUSED:"),
            "`{key}` produced no refusal marker:\n{text}"
        );
        ran += 1;
    }
    // COVERAGE. A table that walked nothing, or an argv builder that silently produced nothing,
    // cannot report a pass.
    assert_eq!(ran, TABLE.len());
    assert!(ran >= 37, "only {ran} commands driven end to end");
}

#[test]
fn the_scaffold_tag_in_help_matches_what_the_command_actually_does() {
    // A status claim is only worth anything if it is checked against behaviour. Every row tagged
    // [SCAFFOLD: sends nothing] must reach a NOT IMPLEMENTED or a refusal; the two LOCAL-ONLY rows
    // are `auth whoami` (which finishes when given a token) and `certs add` (which finishes its
    // plan but still exits non-zero).
    let local_only: Vec<String> = TABLE
        .iter()
        .filter(|c| c.status == Status::LocalOnly)
        .map(|c| c.key())
        .collect();
    assert_eq!(
        local_only,
        vec!["certs add".to_string(), "auth whoami".to_string()]
    );

    for cmd in TABLE.iter().filter(|c| c.status == Status::Scaffold) {
        let key = cmd.key();
        let matches = cli::build().try_get_matches_from(argv(&key)).unwrap();
        let inv = cli::invocation(&matches).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let code = run(&inv, &mut buf);
        let text = String::from_utf8(buf).unwrap();
        assert!(
            code == exit::NOT_IMPLEMENTED || code == exit::UNAUTHORISED,
            "`{key}` is tagged SCAFFOLD but exited {code}:\n{text}"
        );
        assert!(cmd.help_line().contains("[SCAFFOLD: sends nothing]"));
    }
}

#[test]
fn flags_reach_the_invocation() {
    // Guards the marshaller in the other direction: `try_get_one` returning None on an id that
    // DOES exist would make every flag silently inert, and the tests above would not notice
    // because they only assert refusals.
    let m = cli::build()
        .try_get_matches_from([
            "pier",
            "logs",
            "--service",
            "bucket",
            "--id",
            "photos",
            "--coordinator",
            &hex::encode([1u8; 32]),
        ])
        .unwrap();
    let inv = cli::invocation(&m).unwrap();
    assert_eq!(inv.service.as_deref(), Some("bucket"));
    assert_eq!(inv.id.as_deref(), Some("photos"));
    assert_eq!(
        inv.coordinator.as_deref(),
        Some(hex::encode([1u8; 32]).as_str())
    );

    let m = cli::build()
        .try_get_matches_from([
            "pier",
            "certs",
            "add",
            "*.example.com",
            "--ingress",
            "ingress.reach.example",
            "--acme-delegation",
            "acme.box.example",
            "--exclude-tls-alpn",
            "--box-id",
            "7f3a",
        ])
        .unwrap();
    let inv = cli::invocation(&m).unwrap();
    let cert = inv.cert.as_ref().expect("certs add must carry a request");
    assert_eq!(cert.domain, "*.example.com");
    assert_eq!(cert.acme_delegation.as_deref(), Some("acme.box.example"));
    assert!(
        cert.exclude_tls_alpn,
        "--exclude-tls-alpn did not reach the request"
    );
    assert!(
        !cert.caa_enforcement_established,
        "an unset flag must not read as set"
    );
    assert_eq!(
        cert.ca, "letsencrypt.org",
        "the default value did not come through"
    );
    assert_eq!(
        inv.id.as_deref(),
        Some("7f3a"),
        "--box-id scopes the reconfigure"
    );

    // And the wildcard plan it produces really is the DNS-01 one.
    let mut buf: Vec<u8> = Vec::new();
    let code = run(&inv, &mut buf);
    let text = String::from_utf8(buf).unwrap();
    assert_eq!(code, exit::NOT_IMPLEMENTED, "{text}");
    assert!(text.contains("ACME dns-01"), "{text}");
    assert!(text.contains("_acme-challenge.example.com."), "{text}");
    assert!(text.contains("validationmethods=dns-01"), "{text}");
}
