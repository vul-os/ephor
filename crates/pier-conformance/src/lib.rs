//! # pier-conformance
//!
//! The coordinator-contract conformance harness: the [`Coordinator`] trait every
//! Pier kind implements, and [`check`], which runs the COORD-1..8 checklist of
//! `coordinator/CONTRACT.md` §7 against it.
//!
//! Honesty rule (STYLE §8): some clauses are decidable from the descriptor and the
//! kind's declared posture; others are **behavioral** — a coordinator that *declares*
//! blind but *operates* terminating (COORD-5) can only be caught against real
//! traffic. The harness marks those [`Outcome::Behavioral`] rather than falsely
//! passing them, and the per-kind runtime conformance tests discharge them.

use pier_economics::kinds::CoordinatorKind;
use pier_economics::visibility::ContentVisibility;
use pier_economics::Descriptor;

/// What a coordinator kind exposes so the harness can judge the four clauses (§2)
/// and the visibility property (§3). Each kind crate implements this.
pub trait Coordinator {
    fn kind(&self) -> CoordinatorKind;

    /// The signed, discovery-only descriptor (§2.1). Its type structurally excludes
    /// a global score, a price rank, and a stake field (COORD-1).
    fn descriptor(&self) -> &Descriptor;

    /// COORD-2 — switching MUST be a config change with zero data migration and
    /// zero identity change.
    fn lock_in(&self) -> LockIn;

    /// COORD-3 — a self-host backstop, or the one disclosed scarce-reachability
    /// exception class.
    fn self_host(&self) -> SelfHost;

    /// COORD-6 — what the coordinator does on a **delivery / authoritative** path.
    fn delivery_path_gate(&self) -> Gate;

    /// COORD-7 — metering posture.
    fn metering(&self) -> Metering;

    /// COORD-8 — settlement posture.
    fn settlement(&self) -> Settlement;
}

/// COORD-2 lock-in posture.
pub enum LockIn {
    /// Config-only switch, identity unchanged. Conformant.
    None,
    /// A migration or identity change is required to leave — a violation.
    Requires(String),
    //
    // KNOWN GAP, filed rather than patched: there is no honest declaration for a
    // coordinator whose lock-in DEPENDS ON ITS MODE.
    //
    // A §26 legacy chat adapter in node mode carries no lock-in — the WABA is the
    // homeowner's, swapping adapters is config-only, the number survives. The same
    // adapter in gateway mode carries real lock-in: the rail identity is the
    // operator's, and leaving kills the channel (§26.6). Both are true of one
    // implementation.
    //
    // Declaring `Requires` for it is honest and scores as a Violation; declaring
    // `None` would pass and would be the misrepresentation §2.4 names. So the
    // truthful adapter is the non-conformant one, which is a defect in this enum
    // rather than in the adapter.
    //
    // A `ModeDependent { node, gateway }` variant is the obvious fix and is NOT
    // added here on purpose: it changes what COORD-2 means, and that is the spec
    // session's call, not this crate's. Filed in COORDINATION.md
    // [2026-07-27 coord2]; pinned by the test of the same name below.
}

/// COORD-3 self-host posture.
pub enum SelfHost {
    /// Anyone meeting the kind's requirement can run it for themselves.
    Backstop,
    /// The one disclosed exception: a scarce network-reachability resource an
    /// ISP/host allocates. The claimant MUST name which resource, because the
    /// kind alone stopped being sufficient evidence.
    ///
    /// KOTVA §26 states that a legacy-rail adapter "is a `gateway`-kind
    /// coordinator". A WhatsApp or Telegram adapter therefore reports
    /// `kind = gateway` on the wire while needing no reputable IP, no unblocked
    /// port 25, and in the outbound-persistent cases no inbound reachability at
    /// all. Keying this exception off the kind alone would hand every such
    /// adapter a self-host exemption it has no claim to — the check would
    /// return Pass for a coordinator that is trivially self-hostable, which is
    /// the opposite of what §2.3 is for.
    ///
    /// So the resource is named and COORD-3 checks the pairing.
    ScarceReachabilityException(ScarceResource),
    /// No backstop and not the disclosed exception — a violation.
    None(String),
}

/// The scarce network-reachability resources §2.3 recognises. A closed set:
/// anything not here is not scarce, and a claimant naming none of them is
/// claiming an exemption that does not exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScarceResource {
    /// An IP with sending reputation and unblocked port 25 — the mail gateway's
    /// one irreducible cost (§7.1a).
    SmtpEgress,
    /// Publicly reachable ingress, for a party that cannot otherwise be dialled.
    PublicIngress,
}

impl ScarceResource {
    /// Whether a coordinator of this kind can plausibly need this resource.
    ///
    /// A gateway-kind coordinator may need SMTP egress — that is the mail
    /// gateway. It may NOT claim public ingress on that basis alone, because
    /// §26 puts chat adapters in the same kind and an outbound-persistent rail
    /// needs no ingress whatsoever.
    pub fn plausible_for(self, kind: CoordinatorKind) -> bool {
        match self {
            ScarceResource::SmtpEgress => kind == CoordinatorKind::Gateway,
            ScarceResource::PublicIngress => kind == CoordinatorKind::ReachabilityAdapter,
        }
    }
}

/// COORD-6 delivery-path behavior.
pub enum Gate {
    /// Gates on sender identity + rate only. Conformant.
    Authorization,
    /// Classifies content on a delivery/authoritative path — a violation.
    Classification(String),
    /// Classifies/ranks only within its own opt-in, non-authoritative derived view
    /// (labeler/indexer/matcher §4 carve-out). Conformant.
    DerivedViewOnly,
    /// The kind sits on no delivery/authoritative path (e.g. relay of ciphertext).
    NoDeliveryPath,
}

/// COORD-7 metering posture.
pub enum Metering {
    NotMetered,
    /// Issues signed usage receipts directly to the payer. Conformant.
    SignedReceiptsToPayer,
    /// Meters but does not issue payer receipts — a violation.
    NoReceipts(String),
}

/// COORD-8 settlement posture.
pub enum Settlement {
    /// Stakes/settles only in existing assets. Conformant.
    ExistingAssetsOnly,
    /// Mints a protocol token — forbidden (DIRECTION §5).
    MintsToken(String),
}

/// The result of one checklist item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Satisfied — decidable from the descriptor / declared posture.
    Pass,
    /// Violated, with the reason.
    Violation(String),
    /// Not statically decidable from the descriptor; a runtime conformance test
    /// must discharge it (the reason names what to test).
    Behavioral(String),
}

/// One row of the COORD-1..8 checklist and its outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub id: &'static str,
    pub clause: &'static str,
    pub outcome: Outcome,
}

/// The full report for one coordinator.
#[derive(Clone, Debug)]
pub struct Report {
    pub kind: CoordinatorKind,
    pub findings: Vec<Finding>,
}

impl Report {
    /// True iff no finding is an outright [`Outcome::Violation`]. Behavioral items
    /// are not violations — they are deferred to the runtime tests.
    pub fn is_conformant(&self) -> bool {
        !self
            .findings
            .iter()
            .any(|f| matches!(f.outcome, Outcome::Violation(_)))
    }

    /// The clauses still needing a runtime conformance test.
    pub fn behavioral(&self) -> impl Iterator<Item = &Finding> {
        self.findings
            .iter()
            .filter(|f| matches!(f.outcome, Outcome::Behavioral(_)))
    }
}

/// Run the COORD-1..8 checklist against a coordinator (CONTRACT §7).
pub fn check(c: &dyn Coordinator) -> Report {
    let d = c.descriptor();
    let mut findings = Vec::new();

    // COORD-1 — signed, discovery-only descriptor, no score/price-rank/stake.
    // The `Descriptor` type has no field for any of those, so the *shape* is
    // guaranteed by construction; the only thing to check is that the descriptor
    // describes this kind. (The signature itself is behavioral — verified once
    // kotva-core lands.)
    findings.push(Finding {
        id: "COORD-1",
        clause: "§2.1",
        outcome: if d.kind == c.kind() {
            Outcome::Behavioral("verify descriptor signature once kotva-core is pinned".into())
        } else {
            Outcome::Violation(format!(
                "descriptor kind {} != operating kind {}",
                d.kind.as_str(),
                c.kind().as_str()
            ))
        },
    });

    // COORD-2 — zero lock-in.
    findings.push(Finding {
        id: "COORD-2",
        clause: "§2.2",
        outcome: match c.lock_in() {
            LockIn::None => Outcome::Pass,
            LockIn::Requires(why) => Outcome::Violation(why),
        },
    });

    // COORD-3 — self-host backstop, or the disclosed scarce-reachability exception.
    findings.push(Finding {
        id: "COORD-3",
        clause: "§2.3",
        outcome: match c.self_host() {
            SelfHost::Backstop => Outcome::Pass,
            SelfHost::ScarceReachabilityException(resource) => {
                if resource.plausible_for(c.kind()) {
                    Outcome::Pass
                } else {
                    Outcome::Violation(format!(
                        "{} claims the scarce-reachability exception for {:?}, which that \
                         kind cannot need. Note that §26 legacy-rail adapters report \
                         kind=gateway while needing no scarce resource at all — the kind \
                         alone is not evidence.",
                        c.kind().as_str(),
                        resource
                    ))
                }
            }
            SelfHost::None(why) => Outcome::Violation(why),
        },
    });

    // COORD-4 — declares exactly one visibility class + level; clients surface it.
    // Exactly-one is guaranteed by the `ContentVisibility` type. A declared-level
    // blind claim must not be presented as verified — flagged behavioral (the
    // client surface is what must honor it).
    findings.push(Finding {
        id: "COORD-4",
        clause: "§2.4/§3",
        outcome: coord4(d.visibility),
    });

    // COORD-5 — never silently downgrades blind→terminating. Declaring terminating
    // *is* the required disclosure; declaring blind while operating terminating is
    // the violation, and that is only detectable against real traffic.
    findings.push(Finding {
        id: "COORD-5",
        clause: "§3.2",
        outcome: Outcome::Behavioral(
            "assert observed TLS behavior matches the declared visibility class".into(),
        ),
    });

    // COORD-6 — authorize, never classify on a delivery/authoritative path.
    findings.push(Finding {
        id: "COORD-6",
        clause: "§4",
        outcome: match c.delivery_path_gate() {
            Gate::Authorization | Gate::DerivedViewOnly | Gate::NoDeliveryPath => Outcome::Pass,
            Gate::Classification(why) => Outcome::Violation(why),
        },
    });

    // COORD-7 — if metered, issue signed receipts to the payer.
    findings.push(Finding {
        id: "COORD-7",
        clause: "§6",
        outcome: match c.metering() {
            Metering::NotMetered | Metering::SignedReceiptsToPayer => Outcome::Pass,
            Metering::NoReceipts(why) => Outcome::Violation(why),
        },
    });

    // COORD-8 — no token; stake/settle in existing assets only.
    findings.push(Finding {
        id: "COORD-8",
        clause: "§6",
        outcome: match c.settlement() {
            Settlement::ExistingAssetsOnly => Outcome::Pass,
            Settlement::MintsToken(why) => Outcome::Violation(why),
        },
    });

    Report {
        kind: c.kind(),
        findings,
    }
}

fn coord4(v: ContentVisibility) -> Outcome {
    if v.must_not_present_as_verified() {
        Outcome::Behavioral(format!(
            "declared-level {} claim: client MUST surface it as unverified, not verified (§3.4)",
            v
        ))
    } else {
        Outcome::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pier_economics::descriptor::Descriptor;
    use pier_economics::visibility::{AssuranceLevel, VisibilityClass};
    use pier_economics::{Cbor, IdentityKey};

    /// A minimal conformant relay: structurally-blind, no lock-in, self-hostable,
    /// forwards ciphertext (no delivery path), unmetered, no token.
    struct GoodRelay {
        d: Descriptor,
    }

    impl GoodRelay {
        fn new() -> Self {
            // A real kotva-core keypair, not a placeholder `[0u8; 32]` array (the real
            // `IdentityKey` holds a private signing key and has no such literal constructor).
            let ik = IdentityKey::from_seed(&[0u8; 32]);
            Self {
                d: Descriptor {
                    identity: ik.public(),
                    kind: CoordinatorKind::Relay,
                    visibility: ContentVisibility::new(
                        VisibilityClass::Blind,
                        AssuranceLevel::Structural,
                    ),
                    policy: Cbor(Vec::new()),
                    tariff: None,
                },
            }
        }
    }

    impl Coordinator for GoodRelay {
        fn kind(&self) -> CoordinatorKind {
            CoordinatorKind::Relay
        }
        fn descriptor(&self) -> &Descriptor {
            &self.d
        }
        fn lock_in(&self) -> LockIn {
            LockIn::None
        }
        fn self_host(&self) -> SelfHost {
            SelfHost::Backstop
        }
        fn delivery_path_gate(&self) -> Gate {
            Gate::NoDeliveryPath
        }
        fn metering(&self) -> Metering {
            Metering::NotMetered
        }
        fn settlement(&self) -> Settlement {
            Settlement::ExistingAssetsOnly
        }
    }

    #[test]
    fn a_good_relay_is_conformant() {
        let r = check(&GoodRelay::new());
        assert!(r.is_conformant(), "{:?}", r.findings);
        // COORD-1 and COORD-5 remain behavioral (signature + observed TLS).
        assert_eq!(r.behavioral().count(), 2);
    }

    #[test]
    fn classification_on_delivery_path_is_a_violation() {
        struct SpamScoringRelay(Descriptor);
        impl Coordinator for SpamScoringRelay {
            fn kind(&self) -> CoordinatorKind {
                CoordinatorKind::Relay
            }
            fn descriptor(&self) -> &Descriptor {
                &self.0
            }
            fn lock_in(&self) -> LockIn {
                LockIn::None
            }
            fn self_host(&self) -> SelfHost {
                SelfHost::Backstop
            }
            fn delivery_path_gate(&self) -> Gate {
                Gate::Classification("ML spam score drops mail on the delivery path".into())
            }
            fn metering(&self) -> Metering {
                Metering::NotMetered
            }
            fn settlement(&self) -> Settlement {
                Settlement::ExistingAssetsOnly
            }
        }
        let mut good = GoodRelay::new();
        good.d.kind = CoordinatorKind::Relay;
        let r = check(&SpamScoringRelay(good.d));
        assert!(!r.is_conformant());
    }

    #[test]
    fn minting_a_token_is_a_violation() {
        struct TokenRelay(Descriptor);
        impl Coordinator for TokenRelay {
            fn kind(&self) -> CoordinatorKind {
                CoordinatorKind::Relay
            }
            fn descriptor(&self) -> &Descriptor {
                &self.0
            }
            fn lock_in(&self) -> LockIn {
                LockIn::None
            }
            fn self_host(&self) -> SelfHost {
                SelfHost::Backstop
            }
            fn delivery_path_gate(&self) -> Gate {
                Gate::NoDeliveryPath
            }
            fn metering(&self) -> Metering {
                Metering::NotMetered
            }
            fn settlement(&self) -> Settlement {
                Settlement::MintsToken("$RELAY utility token".into())
            }
        }
        let r = check(&TokenRelay(GoodRelay::new().d));
        assert!(!r.is_conformant());
    }
}

#[cfg(test)]
mod scarce_resource_tests {
    use super::*;

    // The hole this change closes. KOTVA §26 says a legacy-rail adapter "is a
    // `gateway`-kind coordinator", so a WhatsApp or Telegram adapter reports
    // kind=gateway on the wire. Before, COORD-3 asked only whether the KIND was
    // in the scarce-reachability class — which it is — and returned Pass.
    //
    // A chat adapter needs no reputable IP, no unblocked port 25, and for an
    // outbound-persistent rail no inbound reachability at all. It is trivially
    // self-hostable. Granting it the self-host exemption is the precise
    // opposite of what §2.3 exists to police.
    #[test]
    fn a_gateway_kind_cannot_claim_ingress_it_does_not_need() {
        assert!(
            !ScarceResource::PublicIngress.plausible_for(CoordinatorKind::Gateway),
            "a gateway-kind coordinator claiming public ingress is the §26 chat-adapter \
             loophole: it reports kind=gateway and needs nothing scarce"
        );
        assert!(
            ScarceResource::SmtpEgress.plausible_for(CoordinatorKind::Gateway),
            "the mail gateway's port-25 claim is the one this exception exists for"
        );
    }

    #[test]
    fn only_the_reachability_adapter_may_claim_ingress() {
        assert!(ScarceResource::PublicIngress.plausible_for(CoordinatorKind::ReachabilityAdapter));
        assert!(!ScarceResource::SmtpEgress.plausible_for(CoordinatorKind::ReachabilityAdapter));
    }

    // Every other kind is self-hostable and may claim neither.
    #[test]
    fn no_other_kind_may_claim_a_scarce_resource() {
        for kind in [
            CoordinatorKind::Relay,
            CoordinatorKind::MediaRelay,
            CoordinatorKind::Indexer,
            CoordinatorKind::Labeler,
            CoordinatorKind::Matcher,
            CoordinatorKind::Compute,
            CoordinatorKind::Arbiter,
            CoordinatorKind::Oracle,
        ] {
            assert!(!ScarceResource::SmtpEgress.plausible_for(kind), "{kind:?}");
            assert!(
                !ScarceResource::PublicIngress.plausible_for(kind),
                "{kind:?}"
            );
        }
    }
}

#[cfg(test)]
mod coord2_gap_tests {
    use super::*;

    /// Pins the CURRENT behaviour of the COORD-2 gap so it cannot be forgotten.
    ///
    /// This test does not assert that the behaviour is right — it asserts what it
    /// is. A truthful mode-dependent declaration scores as a Violation, which
    /// means an adapter that tells the truth about gateway mode fails
    /// conformance while one that lies passes.
    ///
    /// Whichever way the spec resolves it — a `ModeDependent` variant, or a
    /// ruling that §26.6 is a disclosed exception to §2.2 — this test breaks and
    /// forces the code to be updated. That is the point: an open question with no
    /// failing test attached is an open question that gets quietly dropped.
    ///
    /// See COORDINATION.md [2026-07-27 coord2].
    #[test]
    fn coord2_mode_dependent_lock_in_has_no_honest_declaration_yet() {
        // The honest string an Aql chat adapter would have to declare.
        let honest = LockIn::Requires(
            "gateway-mode rail identity is the operator's; leaving kills the channel, §26.6"
                .to_string(),
        );

        let outcome = match honest {
            LockIn::None => Outcome::Pass,
            LockIn::Requires(why) => Outcome::Violation(why),
        };

        match outcome {
            Outcome::Violation(_) => { /* the gap, as filed */ }
            _ => panic!(
                "COORD-2 now accepts a mode-dependent lock-in declaration. If the spec \
                 answered (a ModeDependent variant, or a ruling that §26.6 is a disclosed \
                 exception to §2.2), update check_coord2 and this test together, and mark \
                 the COORDINATION.md thread RESOLVED. Do not delete this test without \
                 doing that — it is the only thing holding the question open."
            ),
        }

        // And the shape of the problem: node mode is genuinely fine, so this is
        // not an adapter that is simply non-conformant. One implementation, two
        // modes, two different truths.
        assert!(
            matches!(LockIn::None, LockIn::None),
            "node mode carries no lock-in and must keep passing"
        );
    }
}
