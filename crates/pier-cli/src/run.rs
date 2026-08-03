//! Dispatch — resolve the selected command to a §5.1 scope and a §5.2 ability, check
//! authorisation, then refuse honestly.
//!
//! The order is deliberate and is the whole safety property of this file. A scaffold command
//! **still** resolves its resource and ability and **still** runs the full authorisation check
//! before it refuses, so the parts that are real get exercised on every invocation rather than
//! only in tests. What it never does is print an outcome. There is no code path here that emits
//! "box created", "certificate issued", or an empty-but-successful list — the two ways this
//! program can end are a local result it genuinely computed, or a refusal naming what is missing.

use std::path::PathBuf;

use kotva_depot::{Ability, ResourceRef, Service};

use crate::auth::{self, AuthError, Authorisation, ENV_COORDINATOR};
use crate::certs::{self, CertPlan, CertRequest, Tier};
use crate::table::{lookup, Action, Cmd, Status};

/// Process exit codes. Distinct, because "you are not authorised" and "this is not built" are
/// different facts and a script should be able to tell them apart.
pub mod exit {
    /// The command produced the local result it promised.
    pub const OK: i32 = 0;
    /// Usage error (clap handles most of these itself).
    pub const USAGE: i32 = 2;
    /// The command is not implemented. Nothing was sent.
    pub const NOT_IMPLEMENTED: i32 = 3;
    /// Authorisation refused, fail-closed. Nothing was sent.
    pub const UNAUTHORISED: i32 = 4;
}

/// Everything one invocation needs, already extracted from clap.
pub struct Invocation {
    /// The table key, e.g. `"box destroy"`.
    pub key: String,
    /// `--token`.
    pub token: Option<PathBuf>,
    /// `--coordinator`, hex.
    pub coordinator: Option<String>,
    /// Instance id, where the command takes one.
    pub id: Option<String>,
    /// `--service`, for `logs`.
    pub service: Option<String>,
    /// The `certs add` request, when that is the command.
    pub cert: Option<CertRequest>,
}

/// The §5.1 scope a command acts on.
///
/// A command that names an instance narrows to it; one that does not stays at
/// `depot:<service>/*`, and `status` sits at `depot:*`. Narrowing here is not a nicety: the token
/// check that follows uses [`ResourceRef::covers`], so asking for more scope than you need is how
/// a request gets refused by a correctly-attenuated token.
pub fn scope(cmd: &Cmd, inv: &Invocation) -> Result<ResourceRef, String> {
    let service = match cmd.service() {
        Some(s) => Some(s),
        None => match inv.service.as_deref() {
            Some(s) => Some(Service::from_str(s).ok_or_else(|| {
                format!("unknown elemental {s:?} — the §3 registry is closed to four")
            })?),
            None => None,
        },
    };
    Ok(match (service, inv.id.as_deref()) {
        (Some(s), Some(id)) => ResourceRef::Instance(s, id.to_string()),
        (Some(s), None) => ResourceRef::AllInstances(s),
        (None, _) => ResourceRef::AllServices,
    })
}

/// Resolve the coordinator `IK` from `--coordinator` or `$PIER_COORDINATOR`, refusing if neither.
fn coordinator(inv: &Invocation) -> Result<Vec<u8>, AuthError> {
    let hex_ik = match &inv.coordinator {
        Some(h) => h.clone(),
        None => std::env::var(ENV_COORDINATOR).map_err(|_| AuthError::NoCoordinator)?,
    };
    auth::parse_coordinator(&hex_ik)
}

/// Run one invocation, writing to `out`. Returns the process exit code.
pub fn run(inv: &Invocation, out: &mut dyn std::io::Write) -> i32 {
    let Some(cmd) = lookup(&inv.key) else {
        let _ = writeln!(out, "error: no such command {:?}", inv.key);
        return exit::USAGE;
    };

    match cmd.action {
        Action::Local => run_local(cmd, inv, out),
        Action::Depot { ability, .. } => run_depot(cmd, inv, ability, out),
    }
}

fn run_local(cmd: &Cmd, inv: &Invocation, out: &mut dyn std::io::Write) -> i32 {
    match cmd.key().as_str() {
        "auth whoami" => match auth::load(inv.token.as_deref()) {
            Ok(a) => {
                print_whoami(&a, out);
                exit::OK
            }
            Err(e) => {
                refuse(out, &e.to_string());
                exit::UNAUTHORISED
            }
        },
        // `auth token` is a scaffold, and the important part is what it refuses to do instead.
        _ => {
            not_implemented(cmd, out);
            exit::NOT_IMPLEMENTED
        }
    }
}

fn run_depot(cmd: &Cmd, inv: &Invocation, ability: Ability, out: &mut dyn std::io::Write) -> i32 {
    let resource = match scope(cmd, inv) {
        Ok(r) => r,
        Err(e) => {
            refuse(out, &e);
            return exit::USAGE;
        }
    };

    // `certs add` is LOCAL-ONLY: it produces a real plan without any coordinator. Compute it
    // FIRST, so a missing token does not hide the one part of this command that works. The DEPOT
    // `reconfigure` it would eventually need is still named below as not implemented.
    if cmd.status == Status::LocalOnly && cmd.key() == "certs add" {
        let Some(req) = &inv.cert else {
            refuse(out, "internal: certs add invoked without a request");
            return exit::USAGE;
        };
        return match certs::plan(req) {
            Ok(p) => {
                print_cert_plan(&p, &resource, out);
                // Non-zero on purpose. `certs add` did not add a certificate; a zero exit would
                // tell every script wrapping this that it did.
                exit::NOT_IMPLEMENTED
            }
            Err(e) => {
                refuse(out, &e.to_string());
                exit::USAGE
            }
        };
    }

    // The full authorisation path, run for real even though the request will not be sent.
    let ik = match coordinator(inv) {
        Ok(k) => k,
        Err(e) => {
            refuse(out, &e.to_string());
            return exit::UNAUTHORISED;
        }
    };
    let authz = match auth::load(inv.token.as_deref()) {
        Ok(a) => a,
        Err(e) => {
            refuse(out, &e.to_string());
            return exit::UNAUTHORISED;
        }
    };
    if let Err(e) = authz.authorise(&ik, &resource, ability) {
        refuse(out, &e.to_string());
        return exit::UNAUTHORISED;
    }

    let _ = writeln!(
        out,
        "authorised: {} on {} at coordinator {}",
        ability.as_str(),
        resource.to_wire(),
        hex::encode(&ik)
    );
    if ability.requires_separate_delegation() {
        let _ = writeln!(
            out,
            "note: `{}` is the §5.2 privilege cliff — it subsumes nearly every other ability, and \
             a token carrying it must have been delegated deliberately, never as a side effect of \
             `provision` or `reconfigure`.",
            ability.as_str()
        );
    }
    not_implemented(cmd, out);
    exit::NOT_IMPLEMENTED
}

fn refuse(out: &mut dyn std::io::Write, msg: &str) {
    let _ = writeln!(out, "REFUSED: {msg}");
}

fn not_implemented(cmd: &Cmd, out: &mut dyn std::io::Write) {
    let _ = writeln!(out, "\nNOT IMPLEMENTED — `pier {}` did nothing.", cmd.key());
    let _ = writeln!(out, "Missing: {}", cmd.missing);
    if let Some(a) = cmd.ability() {
        let _ = writeln!(
            out,
            "\nFor reference, the request it WOULD have carried (this was not sent):\n  \
             ability: {}",
            a.as_str()
        );
    }
}

fn print_whoami(a: &Authorisation, out: &mut dyn std::io::Write) {
    let _ = writeln!(
        out,
        "token:       §18.7.3 CapabilityToken, signature VERIFIED"
    );
    let _ = writeln!(out, "issuer:      {}", hex::encode(&a.token.iss));
    let _ = writeln!(out, "audience:    {}", hex::encode(&a.token.aud));
    let _ = writeln!(
        out,
        "valid:       nbf={} exp={} (ms epoch)",
        a.token.nbf, a.token.exp
    );
    let _ = writeln!(
        out,
        "parent:      {}",
        match &a.token.prnt {
            Some(p) => hex::encode(p.as_bytes()),
            None => "(none — rooted at its own issuer)".to_string(),
        }
    );
    let _ = writeln!(out, "grants:      {}", a.render());
    let unbound = a.grants.iter().filter(|g| g.coordinator.is_none()).count();
    if unbound > 0 {
        let _ = writeln!(
            out,
            "\nWARNING: {unbound} of {} grants carry no `depot:coordinator` caveat. §5.1 requires \
             it, and `pier` will refuse to present those grants to any coordinator — a token that \
             names no coordinator is valid at every DEPOT gateway, which is the confused-deputy \
             hole the caveat closes.",
            a.grants.len()
        );
    }
    let _ = writeln!(
        out,
        "\nNOTE: this verified the token's OWN signature and parsed every grant through the closed \
         §5.2 registry. It did NOT walk the delegation chain to a trusted root, and did not check \
         revocations — both need parent tokens and a root this CLI has no store for."
    );
}

fn print_cert_plan(p: &CertPlan, resource: &ResourceRef, out: &mut dyn std::io::Write) {
    let _ = writeln!(
        out,
        "PLAN ONLY — NO CERTIFICATE WAS ISSUED, AND NO KEY EXISTS YET.\n"
    );
    let _ = writeln!(
        out,
        "domain:      {}{}",
        p.domain,
        if p.wildcard { "  (wildcard)" } else { "" }
    );
    let _ = writeln!(out, "tier:        {}", p.tier.label());
    let _ = writeln!(out, "key holder:  {}", p.tier.key_holder());
    let _ = writeln!(out, "challenge:   ACME {}", p.challenge.as_str());
    let _ = writeln!(out, "visibility:  {}", p.tier.visibility());
    let _ = writeln!(out, "assurance:   {}", p.assurance.as_str());
    let _ = writeln!(out, "  because:   {}", p.assurance_reason);
    let _ = writeln!(
        out,
        "scope:       {} (the eventual `reconfigure`)",
        resource.to_wire()
    );

    let _ = writeln!(
        out,
        "\nTIERS NOT OFFERED BY THIS COMMAND (§3.4), so you can see the trade:"
    );
    for t in Tier::ALL {
        if t.offered_here() {
            continue;
        }
        let _ = writeln!(
            out,
            "  - {}: DNS work {}, key held by {}",
            t.label(),
            t.dns_work(),
            t.key_holder()
        );
    }

    let _ = writeln!(out, "\nDNS RECORDS TO PUBLISH ({}):", p.records.len());
    for r in &p.records {
        let _ = writeln!(out, "\n  {}  {}  {}", r.name, r.kind, r.value);
        let _ = writeln!(out, "      why: {}", r.why);
    }

    let _ = writeln!(out, "\nWHAT IS NOT IMPLEMENTED:");
    for line in CertPlan::not_implemented() {
        let _ = writeln!(out, "\n  {line}");
    }
    let _ = writeln!(
        out,
        "\nExiting non-zero: `certs add` did not add a certificate, and a zero exit would tell \
         anything wrapping this command that it did."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::TABLE;

    fn inv(key: &str) -> Invocation {
        Invocation {
            key: key.to_string(),
            token: None,
            coordinator: None,
            id: None,
            service: None,
            cert: None,
        }
    }

    fn run_capturing(i: &Invocation) -> (i32, String) {
        let mut buf: Vec<u8> = Vec::new();
        let code = run(i, &mut buf);
        (code, String::from_utf8(buf).unwrap())
    }

    #[test]
    fn no_command_ever_reports_success_it_did_not_achieve() {
        // Walk EVERY row. The only zero exits allowed are LocalOnly rows that genuinely finish,
        // and `certs add` is deliberately not one of them.
        std::env::remove_var(crate::auth::ENV_TOKEN);
        std::env::remove_var(ENV_COORDINATOR);
        let mut checked = 0usize;
        for cmd in TABLE {
            let mut i = inv(&cmd.key());
            if cmd.key() == "certs add" {
                i.cert = Some(CertRequest {
                    domain: "app.example.com".into(),
                    ingress: "ingress.reach.example".into(),
                    acme_delegation: None,
                    ca: "letsencrypt.org".into(),
                    acme_account: None,
                    exclude_tls_alpn: false,
                    caa_enforcement_established: false,
                });
            }
            let (code, text) = run_capturing(&i);
            assert_ne!(
                code,
                exit::OK,
                "`{}` exited 0 having done nothing",
                cmd.key()
            );
            // Structural, not lexical: a nonzero exit is the teeth, and every run must carry an
            // explicit marker saying which kind of nothing happened. (An earlier version of this
            // grepped for words like "created" and "issued" and flagged the NOT-IMPLEMENTED prose
            // that says a key is *not* generated — a check that punishes honest disclosure while
            // a smoothly-worded fake success would slip past it.)
            assert!(
                text.contains("NOT IMPLEMENTED") || text.contains("REFUSED:"),
                "`{}` produced output with no refusal marker:\n{text}",
                cmd.key()
            );
            let lower = text.to_lowercase();
            for forbidden in ["successfully", "operation complete", "done."] {
                assert!(
                    !lower.contains(forbidden),
                    "`{}` printed a success claim {forbidden:?}:\n{text}",
                    cmd.key()
                );
            }
            checked += 1;
        }
        // Coverage assertion: a loop over an empty table would otherwise pass silently.
        assert_eq!(checked, TABLE.len());
        assert!(checked >= 35, "the table shrank to {checked} rows");
    }

    #[test]
    fn a_depot_command_fails_closed_before_it_reaches_not_implemented() {
        std::env::remove_var(crate::auth::ENV_TOKEN);
        std::env::remove_var(ENV_COORDINATOR);
        let (code, text) = run_capturing(&inv("box destroy"));
        assert_eq!(code, exit::UNAUTHORISED, "{text}");
        assert!(text.starts_with("REFUSED:"), "{text}");
        // The refusal must be about the coordinator binding or the missing token — never a
        // suggestion to proceed without one.
        assert!(
            text.contains("coordinator") || text.contains("CapabilityToken"),
            "{text}"
        );
        assert!(!text.to_lowercase().contains("api key"), "{text}");
    }

    #[test]
    fn scope_narrows_to_the_named_instance_and_widens_only_when_none_is_named() {
        let cmd = lookup("box destroy").unwrap();
        let mut i = inv("box destroy");
        assert_eq!(scope(cmd, &i).unwrap().to_wire(), "depot:box/*");
        i.id = Some("7f3a".into());
        assert_eq!(scope(cmd, &i).unwrap().to_wire(), "depot:box/7f3a");

        let st = lookup("status").unwrap();
        assert_eq!(scope(st, &inv("status")).unwrap().to_wire(), "depot:*");

        let logs = lookup("logs").unwrap();
        let mut li = inv("logs");
        li.service = Some("bucket".into());
        li.id = Some("photos".into());
        assert_eq!(scope(logs, &li).unwrap().to_wire(), "depot:bucket/photos");
        li.service = Some("vm".into());
        assert!(
            scope(logs, &li).is_err(),
            "a coined elemental must be refused"
        );
    }

    #[test]
    fn certs_add_prints_the_plan_and_still_exits_non_zero() {
        let mut i = inv("certs add");
        i.cert = Some(CertRequest {
            domain: "app.example.com".into(),
            ingress: "ingress.reach.example".into(),
            acme_delegation: None,
            ca: "letsencrypt.org".into(),
            acme_account: None,
            exclude_tls_alpn: false,
            caa_enforcement_established: false,
        });
        let (code, text) = run_capturing(&i);
        assert_eq!(code, exit::NOT_IMPLEMENTED);
        assert!(text.contains("NO CERTIFICATE WAS ISSUED"), "{text}");
        assert!(text.contains("THE BOX"), "{text}");
        assert!(text.contains("CAA"), "{text}");
        assert!(text.contains("WHAT IS NOT IMPLEMENTED"), "{text}");
    }

    #[test]
    fn whoami_refuses_when_there_is_no_token() {
        std::env::remove_var(crate::auth::ENV_TOKEN);
        let (code, text) = run_capturing(&inv("auth whoami"));
        assert_eq!(code, exit::UNAUTHORISED);
        assert!(text.contains("no CapabilityToken"), "{text}");
    }
}
