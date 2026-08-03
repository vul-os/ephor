//! Pier's own DEPOT §5.2 control vocabulary — the verb gate an `infra-service` coordinator applies
//! before anything else looks at a request.
//!
//! # Why this is written out longhand instead of re-exported
//!
//! `profiles/cloud.md` §5.2 is a **closed** registry of exact strings, and the whole point of closing
//! it is that one open-source client can drive *any* conformant gateway. That guarantee is only worth
//! something if there is more than one implementation of the table, and if they are checked against
//! each other. So this is pier's **independent** transcription of the §5.2 table, taken from the
//! profile document rather than from `kotva-depot`'s `Ability` enum. Re-exporting the reference
//! registry would make [`crate`]'s side of the agreement true by construction and the
//! `ability-conformance` probe that checks it a tautology — see
//! `tests/ability_conformance.rs`, which runs kotva's probe against *this* table.
//!
//! # Two rules, both fail-closed
//!
//! - **An unknown ability is REFUSED, never mapped.** §5.2: a coordinator receiving an ability
//!   outside the registry MUST refuse (`FAIL_CLOSED_BLOCK`, §21) and MUST NOT map it onto a
//!   similar-sounding one. `terminate` is not `destroy`; a new verb is a registry addition, not an
//!   operator's coinage. Silent aliasing is how two conformant implementations diverge without
//!   either noticing, and unlike an unrecognised *caveat* — which §18.7.3 fails closed on by
//!   construction — a free-text `ability` fails **open** unless someone writes this check.
//! - **A verb is scoped to its elemental.** `attach` is a `volume` verb; a `bucket` that answers to
//!   it has one flat table where the profile has four columns. Matching is byte equality: no
//!   trimming, no case folding, no prefix matching, no namespace stripping.
//!
//! # What this is not
//!
//! This is the **vocabulary** gate, not authorisation and not an implementation of the operations.
//! [`accepts_ability`] answering `true` means "this is a recognised verb for this elemental here" —
//! whether the caller's `CapabilityToken` actually carries it, and whether the coordinator then does
//! the work, are separate questions this crate's scaffold disclosure still applies to.

/// The seven common lifecycle verbs, applicable to all four elementals (§5.2).
const COMMON: [&str; 7] = [
    "provision",
    "inspect",
    "list",
    "reconfigure",
    "observe",
    "export",
    "destroy",
];

/// `box` — a managed node (§5.2). `console` is the privilege cliff and MUST be separately
/// delegable; it is a *recognised* verb here, which is a different question from whether a token
/// carries it.
const BOX: [&str; 5] = ["start", "stop", "restart", "snapshot", "console"];

/// `volume` — block storage (§5.2). `snapshot` is deliberately shared with `box`; nothing else here
/// is valid anywhere else.
const VOLUME: [&str; 4] = ["attach", "detach", "resize", "snapshot"];

/// `bucket` — object storage (§5.2). `serve` toggles public serving (the CDN fold, §3.7).
const BUCKET: [&str; 4] = ["read", "write", "delete", "serve"];

/// `edge-fn` — serverless compute (§5.2), which is also where hosted inference lands (§3.7).
const EDGE_FN: [&str; 3] = ["deploy", "invoke", "rollback"];

/// The §21 error an unknown ability MUST be refused with (§5.2).
pub const FAIL_CLOSED_BLOCK: &str = "FAIL_CLOSED_BLOCK";

/// Why a control request was refused. There is exactly one shape of refusal, deliberately: a
/// coordinator that distinguished "unknown verb" from "wrong elemental" in its *reply* would hand a
/// prober a vocabulary oracle, and both are the same `FAIL_CLOSED_BLOCK` on the wire.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AbilityRefused;

impl core::fmt::Display for AbilityRefused {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{FAIL_CLOSED_BLOCK}: ability outside the closed §5.2 registry for this elemental — \
             refused, never mapped onto a near-match"
        )
    }
}

impl std::error::Error for AbilityRefused {}

/// The per-elemental verbs for a §3 elemental wire string, or `None` for an unknown elemental.
///
/// An unrecognised `service` fails closed with the same force as an unrecognised ability: "there are
/// exactly four" is the property the profile rests on (§3), so a fifth is refused rather than
/// guessed at.
fn per_elemental(service: &str) -> Option<&'static [&'static str]> {
    Some(match service {
        "box" => &BOX,
        "volume" => &VOLUME,
        "bucket" => &BUCKET,
        "edge-fn" => &EDGE_FN,
        _ => return None,
    })
}

/// Whether `ability` is a recognised §5.2 verb **for this elemental** at this coordinator.
///
/// Byte equality against the closed registry. Everything else — `terminate`, `DESTROY`,
/// `"destroy "`, `depot:destroy`, `destroyx`, `attach` on a `bucket` — is `false`.
pub fn accepts_ability(service: &str, ability: &str) -> bool {
    match per_elemental(service) {
        Some(per) => COMMON.contains(&ability) || per.contains(&ability),
        None => false,
    }
}

/// [`accepts_ability`] as a fail-closed result, which is the shape a request handler wants.
pub fn authorize_ability(service: &str, ability: &str) -> Result<(), AbilityRefused> {
    if accepts_ability(service, ability) {
        Ok(())
    } else {
        Err(AbilityRefused)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_common_seven_are_accepted_by_every_elemental() {
        let mut checked = 0;
        for service in ["box", "volume", "bucket", "edge-fn"] {
            for verb in COMMON {
                assert!(accepts_ability(service, verb), "{service}/{verb}");
                checked += 1;
            }
        }
        assert_eq!(checked, 28, "4 elementals x 7 common verbs");
    }

    #[test]
    fn per_elemental_verbs_are_scoped_to_their_elemental() {
        assert!(accepts_ability("volume", "attach"));
        assert!(!accepts_ability("bucket", "attach"));
        assert!(accepts_ability("box", "console"));
        assert!(!accepts_ability("edge-fn", "console"));
        assert!(accepts_ability("edge-fn", "deploy"));
        assert!(!accepts_ability("box", "deploy"));
        // snapshot is the one verb §5.2 shares between the two stateful elementals.
        assert!(accepts_ability("box", "snapshot"));
        assert!(accepts_ability("volume", "snapshot"));
        assert!(!accepts_ability("bucket", "snapshot"));
    }

    #[test]
    fn terminate_is_not_destroy() {
        // The coinage §5.2 names by hand.
        assert!(!accepts_ability("box", "terminate"));
        assert_eq!(authorize_ability("box", "terminate"), Err(AbilityRefused));
        assert!(authorize_ability("box", "destroy").is_ok());
    }

    #[test]
    fn matching_is_byte_equality() {
        for coined in [
            "DESTROY",
            "Destroy",
            "destroy ",
            " destroy",
            "destroyx",
            "destro",
            "depot:destroy",
            "",
        ] {
            assert!(!accepts_ability("box", coined), "{coined:?}");
        }
    }

    #[test]
    fn an_unknown_elemental_fails_closed() {
        // A fifth elemental is a breaking change to §3, not something to guess at.
        assert!(!accepts_ability("database", "provision"));
        assert!(!accepts_ability("compute", "provision"));
        assert!(!accepts_ability("", "provision"));
        assert!(!accepts_ability("Box", "provision"));
    }

    #[test]
    fn the_refusal_names_the_section_21_error() {
        assert!(AbilityRefused.to_string().contains(FAIL_CLOSED_BLOCK));
    }
}
