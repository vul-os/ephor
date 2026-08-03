//! `pier certs add` — one command, and the **box** keeps the private key.
//!
//! # Why this command exists in this shape
//!
//! Fly's `flyctl certs add` is one command and the operator ends up holding the certificate. That
//! is the whole custody story: the party you are renting from can read your TLS traffic, and can
//! mint a certificate for your name whenever it likes. DEPOT §3.4 says the ergonomics and the
//! custody are **separable**, and its four-tier table is where that is made explicit:
//!
//! | Tier | User's DNS work | Who holds the TLS key | Visibility |
//! |---|---|---|---|
//! | Operator vanity — `you.gateway.example` | none | operator | `declared` |
//! | Own domain, operator-driven ACME | one `CNAME` | operator | `declared` |
//! | **Own domain, box-held key** | one `CNAME` (+ one `CNAME _acme-challenge` for a wildcard) | **the box** | `blind-routing`; `structural` **iff** REACH-1a's CAA precondition holds |
//! | Operator-hosted zone (NS delegation) | delegate the zone | operator | `declared` |
//!
//! and then makes the point that decides this command's design: *"the middle two tiers differ only
//! in who holds the key, not in how much the user types."* Both are one `CNAME`. So the
//! box-held-key tier costs the user **nothing extra in typing** and buys them a key the operator
//! never sees. `pier certs add` implements that tier and no other. It does not offer the two
//! operator-key tiers at all — not because they are forbidden (§3.4 encourages a zero-config
//! default), but because a CLI that silently picks the tier where the operator holds the key,
//! while the help text talks about custody, is precisely the misrepresentation §3.4's last
//! sentence bans: *"an operator MUST NOT present a tier where it holds the key as though it were
//! the tier where the box does."*
//!
//! It also never offers NS delegation, which §3.4 makes normative: *"A gateway MUST NOT require
//! delegation of a user's DNS zone."* One record is a delegation you can revoke by deleting a line.
//! A zone is a captor.
//!
//! # The tension between REACH-1a and REACH-2a, which this command surfaces rather than hides
//!
//! REACH-2a makes **TLS-ALPN-01** the default: the CA dials the public name on 443, the adapter
//! forwards that handshake down the same SNI-passthrough path as ordinary traffic, and the box
//! answers it. No DNS write at all, so the adapter never becomes a co-writer of the zone.
//!
//! REACH-1a then observes that this same property is what makes TLS-ALPN-01 hijackable **by the
//! in-path adapter**: it routes that handshake, so it can answer the challenge itself, under the
//! same CAA-permitted CA but its own ACME account, and mint a MITM certificate. It therefore says
//! a zone claiming `structural` SHOULD pin CAA `validationmethods` to *exclude* `tls-alpn-01`.
//!
//! Those two pull opposite ways, and the resolution costs a DNS record: excluding TLS-ALPN-01
//! forces DNS-01, which needs the `_acme-challenge` delegation that REACH-2a otherwise reserves for
//! wildcards. This module implements both and makes you choose (`--exclude-tls-alpn`), rather than
//! picking one and calling the result `structural` either way.

/// The §3.4 tiers. All four are named because a user needs to see which one they are being handed
/// and what the others would have cost — a tier table with only the chosen row in it is a claim,
/// not a disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// `you.gateway.example` — no DNS work, operator holds the key.
    OperatorVanity,
    /// Own domain, operator-driven ACME — one `CNAME`, operator holds the key.
    OwnDomainOperatorKey,
    /// Own domain, **box-held key** — one `CNAME`, and the operator never sees the key.
    OwnDomainBoxKey,
    /// NS delegation — the whole zone. Offered by nobody here.
    OperatorHostedZone,
}

impl Tier {
    /// Every tier, in §3.4 table order.
    pub const ALL: [Tier; 4] = [
        Tier::OperatorVanity,
        Tier::OwnDomainOperatorKey,
        Tier::OwnDomainBoxKey,
        Tier::OperatorHostedZone,
    ];

    /// The §3.4 row label.
    pub fn label(self) -> &'static str {
        match self {
            Tier::OperatorVanity => "Operator vanity (you.gateway.example)",
            Tier::OwnDomainOperatorKey => "Own domain, operator-driven ACME",
            Tier::OwnDomainBoxKey => "Own domain, box-held key",
            Tier::OperatorHostedZone => "Operator-hosted zone (NS delegation)",
        }
    }

    /// What the user has to type, per §3.4.
    pub fn dns_work(self) -> &'static str {
        match self {
            Tier::OperatorVanity => "none",
            Tier::OwnDomainOperatorKey => "one CNAME",
            Tier::OwnDomainBoxKey => "one CNAME (+ one CNAME _acme-challenge for a wildcard)",
            Tier::OperatorHostedZone => "delegate the whole zone",
        }
    }

    /// Who ends up with the private key.
    pub fn key_holder(self) -> &'static str {
        match self {
            Tier::OwnDomainBoxKey => "THE BOX",
            _ => "the operator",
        }
    }

    /// The §3.4 visibility column.
    pub fn visibility(self) -> &'static str {
        match self {
            Tier::OperatorVanity => {
                "declared — the operator is sole writer of its own zone and can mint a certificate \
                 for that name at any time; the MITM residual is real and disclosed"
            }
            Tier::OwnDomainOperatorKey => "declared — the operator terminates TLS",
            Tier::OwnDomainBoxKey => {
                "blind-routing; structural IFF REACH-1a's CAA precondition is genuinely met"
            }
            Tier::OperatorHostedZone => "declared; must be optional and must gate nothing",
        }
    }

    /// Whether `pier certs add` will produce this tier. Only one does.
    pub fn offered_here(self) -> bool {
        matches!(self, Tier::OwnDomainBoxKey)
    }
}

/// The ACME challenge type this plan uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Challenge {
    /// RFC 8737, REACH-2a's default. No DNS write; the CA's handshake rides the passthrough path.
    TlsAlpn01,
    /// RFC 8555 DNS-01. Required for a wildcard (CA/Browser Forum bars TLS-ALPN-01 there), and the
    /// only option left once CAA `validationmethods` excludes `tls-alpn-01`.
    Dns01,
}

impl Challenge {
    /// The ACME identifier string.
    pub fn as_str(self) -> &'static str {
        match self {
            Challenge::TlsAlpn01 => "tls-alpn-01",
            Challenge::Dns01 => "dns-01",
        }
    }
}

/// CONTRACT §3.3 assurance for the resulting `blind-routing` claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assurance {
    /// Attested by the operator; a client cannot check it from the wire.
    Declared,
    /// Provable from key placement and ACME account binding — but see REACH-1a: the precondition
    /// is verifiable by the **zone owner**, never by the connecting client.
    Structural,
}

impl Assurance {
    /// The CONTRACT §3.3 label.
    pub fn as_str(self) -> &'static str {
        match self {
            Assurance::Declared => "declared",
            Assurance::Structural => "structural",
        }
    }
}

/// One DNS record the user must publish, with the reason it exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsRecord {
    /// `CNAME`, `CAA`.
    pub kind: &'static str,
    /// Owner name, fully qualified with a trailing dot.
    pub name: String,
    /// The RDATA, rendered as a zone-file value.
    pub value: String,
    /// Why this record is required, in one line.
    pub why: &'static str,
}

/// What the user asked for.
#[derive(Debug, Clone)]
pub struct CertRequest {
    /// The name to certify. A leading `*.` requests a wildcard.
    pub domain: String,
    /// The adapter ingress host the public name should point at (REACH-1 SNI passthrough).
    pub ingress: String,
    /// A zone the **box** controls, for `CNAME _acme-challenge` delegation (REACH-2a).
    pub acme_delegation: Option<String>,
    /// The CA the CAA record permits.
    pub ca: String,
    /// The box's ACME account URI, for the RFC 8657 `accounturi` binding.
    pub acme_account: Option<String>,
    /// Pin CAA `validationmethods` to exclude `tls-alpn-01` (REACH-1a's SHOULD). Forces DNS-01.
    pub exclude_tls_alpn: bool,
    /// The operator asserts it has established that the named CA actually enforces RFC 8657
    /// `accounturi`. Without this, REACH-1a requires `declared`, not `structural`.
    pub caa_enforcement_established: bool,
}

/// Ways the request cannot be turned into a plan. Each one is a refusal to print a flow that
/// would not work.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CertError {
    /// No domain, or one that is not a hostname.
    #[error("not a domain name: {0:?}")]
    BadDomain(String),
    /// `*` somewhere other than the leading label.
    #[error("a wildcard is only valid as the leading label (`*.example.com`), got {0:?}")]
    BadWildcard(String),
    /// No ingress host to point the name at.
    #[error("--ingress is required: the public name has to CNAME to the adapter's ingress host")]
    NoIngress,
    /// DNS-01 is required but no delegation target was given.
    #[error(
        "{reason} requires ACME DNS-01, which needs `--acme-delegation <zone-the-box-controls>`.\n\
         REACH-2a: the box places its own `_acme-challenge` value in a zone IT controls, reached by \
         one CNAME. Without that target there is no way for the box to answer the challenge while \
         still holding the key, and `pier` will not fall back to a tier where the operator holds it."
    )]
    NeedsDelegation {
        /// What forced DNS-01 — a wildcard, or the CAA `validationmethods` pin.
        reason: &'static str,
    },
    /// `structural` was claimed with no `accounturi` to bind.
    #[error(
        "--caa-enforcement-established was given but --acme-account was not.\n\
         REACH-1a's `structural` claim rests on an RFC 8657 `accounturi`-bound CAA record naming \
         the BOX's own ACME account. A bare RFC 8659 CAA names only a permitted CA and does not \
         exclude an in-path adapter completing the challenge under its own account."
    )]
    StructuralWithoutAccount,
}

/// A resolved, printable plan. Producing one issues nothing — see [`CertPlan::not_implemented`].
#[derive(Debug, Clone)]
pub struct CertPlan {
    /// The name being certified, as given.
    pub domain: String,
    /// The base name with any `*.` stripped — the zone apex the CAA record hangs off.
    pub base: String,
    /// Whether a wildcard was requested.
    pub wildcard: bool,
    /// Always [`Tier::OwnDomainBoxKey`]; carried explicitly so the printer cannot drift from it.
    pub tier: Tier,
    /// The ACME challenge this plan uses.
    pub challenge: Challenge,
    /// The assurance the resulting claim is entitled to.
    pub assurance: Assurance,
    /// Why it is that and not the other one.
    pub assurance_reason: String,
    /// Records the user must publish, in the order they should be added.
    pub records: Vec<DnsRecord>,
}

fn valid_label(l: &str) -> bool {
    !l.is_empty()
        && l.len() <= 63
        && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !l.starts_with('-')
        && !l.ends_with('-')
}

/// Turn a request into a plan, or refuse.
pub fn plan(req: &CertRequest) -> Result<CertPlan, CertError> {
    let raw = req.domain.trim().trim_end_matches('.');
    if raw.is_empty() || raw.contains("://") || raw.contains('/') {
        return Err(CertError::BadDomain(req.domain.clone()));
    }
    let wildcard = raw.starts_with("*.");
    let base = if wildcard { &raw[2..] } else { raw };
    if base.contains('*') {
        return Err(CertError::BadWildcard(req.domain.clone()));
    }
    let labels: Vec<&str> = base.split('.').collect();
    if labels.len() < 2 || !labels.iter().all(|l| valid_label(l)) {
        return Err(CertError::BadDomain(req.domain.clone()));
    }
    if req.ingress.trim().is_empty() {
        return Err(CertError::NoIngress);
    }
    if req.caa_enforcement_established && req.acme_account.is_none() {
        return Err(CertError::StructuralWithoutAccount);
    }

    // The challenge, and what forced it.
    let (challenge, forced_by) = if wildcard {
        (Challenge::Dns01, Some("a wildcard certificate"))
    } else if req.exclude_tls_alpn {
        (
            Challenge::Dns01,
            Some("--exclude-tls-alpn (REACH-1a's validationmethods pin)"),
        )
    } else {
        (Challenge::TlsAlpn01, None)
    };
    let delegation = match (challenge, req.acme_delegation.as_deref()) {
        (Challenge::Dns01, None) => {
            return Err(CertError::NeedsDelegation {
                reason: forced_by.unwrap_or("DNS-01"),
            })
        }
        (Challenge::Dns01, Some(z)) => Some(z.trim().trim_end_matches('.').to_string()),
        (Challenge::TlsAlpn01, _) => None,
    };

    let mut records = Vec::new();
    // 1. The one CNAME that is the whole of the user's DNS work in the ordinary case.
    records.push(DnsRecord {
        kind: "CNAME",
        name: format!(
            "{}.",
            if wildcard {
                format!("*.{base}")
            } else {
                base.to_string()
            }
        ),
        value: format!("{}.", req.ingress.trim().trim_end_matches('.')),
        why: "REACH-1: points the public name at the adapter, which SNI-passthrough-routes it to \
              the box without terminating TLS.",
    });
    // 2. The delegation, only when DNS-01 is actually in play.
    if let Some(zone) = &delegation {
        records.push(DnsRecord {
            kind: "CNAME",
            name: format!("_acme-challenge.{base}."),
            value: format!("{zone}."),
            why: "REACH-2a: delegates exactly ONE record so the box can place its own DNS-01 \
                  challenge value. This is a delegation you revoke by deleting a line — never an \
                  NS delegation of the zone, which §3.4 forbids requiring.",
        });
    }
    // 3. CAA — necessary for `structural`, and never sufficient on its own.
    let mut caa = req.ca.trim().to_string();
    if let Some(acct) = &req.acme_account {
        caa.push_str("; accounturi=");
        caa.push_str(acct.trim());
    }
    if req.exclude_tls_alpn {
        caa.push_str("; validationmethods=dns-01");
    }
    records.push(DnsRecord {
        kind: "CAA",
        name: format!("{base}."),
        value: format!("0 {} \"{}\"", if wildcard { "issuewild" } else { "issue" }, caa),
        why: "REACH-1a: an RFC 8657 accounturi-bound CAA record naming the BOX's own ACME account. \
              A bare RFC 8659 CAA restricts only WHICH CA may issue — not the validation method or \
              the account — so it does not stop an in-path adapter completing TLS-ALPN-01 under the \
              same CA with its own account.",
    });

    let (assurance, assurance_reason) = if !req.caa_enforcement_established {
        (
            Assurance::Declared,
            "--caa-enforcement-established was not given. REACH-1a §5.2: an accounturi binding \
             \"restrains only a CA that implements RFC 8657\", and a domain MUST NOT assume the \
             restriction is effective absent explicit indication from that CA. Until that is \
             established for the named CA, the honest label is `declared`."
                .to_string(),
        )
    } else if req.exclude_tls_alpn {
        (
            Assurance::Structural,
            format!(
                "CAA is accounturi-bound to the box's ACME account at {ca}, whose RFC 8657 \
                 enforcement you have stated is established, AND validationmethods excludes \
                 tls-alpn-01 — closing the in-path-adapter challenge-hijack REACH-1a names. \
                 Residual: this precondition is verifiable by YOU, the zone owner, and never by a \
                 connecting client.",
                ca = req.ca.trim()
            ),
        )
    } else {
        (
            Assurance::Structural,
            format!(
                "CAA is accounturi-bound to the box's ACME account at {ca}, whose RFC 8657 \
                 enforcement you have stated is established. NOTE the residual you are accepting: \
                 validationmethods still permits tls-alpn-01, REACH-2a's default and the \
                 in-path-hijackable method — so this rests entirely on {ca} honouring accounturi. \
                 Pass --exclude-tls-alpn to close that (it costs one more CNAME).",
                ca = req.ca.trim()
            ),
        )
    };

    Ok(CertPlan {
        domain: raw.to_string(),
        base: base.to_string(),
        wildcard,
        tier: Tier::OwnDomainBoxKey,
        challenge,
        assurance,
        assurance_reason,
        records,
    })
}

impl CertPlan {
    /// Everything `certs add` does **not** do. Printed verbatim every time, above the plan, so a
    /// user cannot mistake a plan for an issued certificate.
    pub fn not_implemented() -> &'static [&'static str] {
        &[
            "1. ASKING THE BOX TO GENERATE ITS KEY. The whole point of this tier is that the key \
             is generated on the box and never leaves it — so `pier` must ask, and it has no \
             channel to ask over (DEPOT-1's IK-authenticated Noise transport is not built). `pier` \
             does NOT generate a key locally; doing so would hand you the operator-key tier under \
             the box-key tier's name.",
            "2. THE ACME RUN (RFC 8555). No account registration, no newOrder, no challenge \
             response — neither the TLS-ALPN-01 responder on :443 nor the DNS-01 TXT publication — \
             no finalize, and no certificate download. Nothing contacts a CA.",
            "3. PUBLISHING OR CHECKING THE DNS. The records above are printed for you to publish. \
             `pier` does not write them, and does not resolve them afterwards to confirm they took \
             effect. In particular it has NOT verified that the CAA record exists.",
            "4. THE `reconfigure` CALL. Telling the box to start serving the new name is a DEPOT \
             `reconfigure` on `depot:box/<id>`, which needs the same missing transport.",
            "5. CT-LOG MONITORING. REACH-1a says a box SHOULD watch Certificate Transparency for \
             covert issuance of its own names. Not built; that is a detection control for the \
             `declared` residual, and its absence is part of what `declared` means here.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(domain: &str) -> CertRequest {
        CertRequest {
            domain: domain.to_string(),
            ingress: "ingress.reach.example".to_string(),
            acme_delegation: None,
            ca: "letsencrypt.org".to_string(),
            acme_account: None,
            exclude_tls_alpn: false,
            caa_enforcement_established: false,
        }
    }

    #[test]
    fn the_ordinary_case_is_tls_alpn_01_and_needs_no_acme_delegation() {
        let p = plan(&req("app.example.com")).unwrap();
        assert_eq!(p.challenge, Challenge::TlsAlpn01);
        assert_eq!(p.tier, Tier::OwnDomainBoxKey);
        assert!(!p.wildcard);
        // REACH-2a's central property: no DNS write for the challenge at all.
        assert!(
            !p.records
                .iter()
                .any(|r| r.name.starts_with("_acme-challenge")),
            "TLS-ALPN-01 must not ask for an _acme-challenge record: {:?}",
            p.records
        );
        // One CNAME + one CAA, and nothing else.
        assert_eq!(p.records.len(), 2, "{:?}", p.records);
        assert_eq!(p.records[0].kind, "CNAME");
        assert_eq!(p.records[1].kind, "CAA");
        assert!(
            p.records[1].value.contains("issue "),
            "{}",
            p.records[1].value
        );
    }

    #[test]
    fn a_wildcard_forces_dns_01_and_the_one_record_delegation() {
        let mut r = req("*.example.com");
        r.acme_delegation = Some("acme.box.example".into());
        let p = plan(&r).unwrap();
        assert!(p.wildcard);
        assert_eq!(p.challenge, Challenge::Dns01);
        let deleg = p
            .records
            .iter()
            .find(|x| x.name == "_acme-challenge.example.com.")
            .expect("wildcard needs the _acme-challenge delegation");
        assert_eq!(deleg.kind, "CNAME");
        assert_eq!(deleg.value, "acme.box.example.");
        // A wildcard's CAA property is issuewild, not issue.
        assert!(
            p.records
                .iter()
                .any(|x| x.kind == "CAA" && x.value.contains("issuewild")),
            "{:?}",
            p.records
        );
    }

    #[test]
    fn a_wildcard_without_a_delegation_target_refuses_rather_than_downgrading() {
        // The failure mode being prevented: quietly producing the operator-key tier because the
        // box-key tier could not be completed.
        let err = plan(&req("*.example.com")).unwrap_err();
        assert!(matches!(err, CertError::NeedsDelegation { .. }), "{err}");
        assert!(err.to_string().contains("acme-delegation"), "{err}");
    }

    #[test]
    fn excluding_tls_alpn_forces_dns_01_even_without_a_wildcard() {
        // This is the REACH-1a / REACH-2a tension: the SHOULD that hardens the CAA record is
        // exactly the one that takes away REACH-2a's no-DNS-write default.
        let mut r = req("app.example.com");
        r.exclude_tls_alpn = true;
        assert!(matches!(
            plan(&r).unwrap_err(),
            CertError::NeedsDelegation { .. }
        ));
        r.acme_delegation = Some("acme.box.example".into());
        let p = plan(&r).unwrap();
        assert_eq!(p.challenge, Challenge::Dns01);
        assert!(p
            .records
            .iter()
            .any(|x| x.value.contains("validationmethods=dns-01")));
    }

    #[test]
    fn assurance_is_declared_until_the_caa_precondition_is_established() {
        let p = plan(&req("app.example.com")).unwrap();
        assert_eq!(p.assurance, Assurance::Declared);
        assert!(
            p.assurance_reason.contains("RFC 8657"),
            "{}",
            p.assurance_reason
        );
    }

    #[test]
    fn structural_requires_an_accounturi_to_bind() {
        let mut r = req("app.example.com");
        r.caa_enforcement_established = true;
        // Claiming structural with a bare RFC 8659 CAA is the exact overclaim REACH-1a names.
        assert_eq!(plan(&r).unwrap_err(), CertError::StructuralWithoutAccount);

        r.acme_account = Some("https://acme-v02.api.letsencrypt.org/acme/acct/1".into());
        let p = plan(&r).unwrap();
        assert_eq!(p.assurance, Assurance::Structural);
        assert!(p.records.iter().any(|x| x.value.contains("accounturi=")));
        // Structural WITHOUT the validationmethods pin must still disclose the residual.
        assert!(
            p.assurance_reason.contains("tls-alpn-01"),
            "{}",
            p.assurance_reason
        );
    }

    #[test]
    fn only_the_box_key_tier_is_offered() {
        let offered: Vec<Tier> = Tier::ALL.into_iter().filter(|t| t.offered_here()).collect();
        assert_eq!(offered, vec![Tier::OwnDomainBoxKey]);
        assert_eq!(Tier::OwnDomainBoxKey.key_holder(), "THE BOX");
        for t in Tier::ALL {
            if t != Tier::OwnDomainBoxKey {
                assert_eq!(t.key_holder(), "the operator", "{t:?}");
            }
        }
    }

    #[test]
    fn junk_domains_are_refused() {
        for bad in [
            "",
            "  ",
            "localhost",
            "https://example.com",
            "example.com/x",
            "ex*ample.com",
            "-x.example.com",
            "a..b",
        ] {
            assert!(plan(&req(bad)).is_err(), "{bad:?} must be refused");
        }
        assert!(plan(&req("*.*.example.com")).is_err());
        let mut no_ing = req("app.example.com");
        no_ing.ingress = "  ".into();
        assert_eq!(plan(&no_ing).unwrap_err(), CertError::NoIngress);
    }

    #[test]
    fn the_not_implemented_list_is_present_and_names_the_key_generation_gap() {
        let list = CertPlan::not_implemented();
        assert!(list.len() >= 5);
        assert!(list.iter().any(|s| s.contains("GENERATE ITS KEY")));
        assert!(list.iter().any(|s| s.contains("RFC 8555")));
    }
}
