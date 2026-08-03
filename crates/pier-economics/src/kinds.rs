//! Coordinator kinds — all instances of the one contract (CONTRACT §5).
//!
//! Every kind inherits the four conformance clauses (§2) and the content-visibility
//! property (§3) unchanged. `gateway` (KOTVA-Mail §7) and the legacy adapters (§26)
//! are the first fully-worked instances; the rest inherit them.

use crate::visibility::{AssuranceLevel, ContentVisibility, VisibilityClass};

/// The coordinator kinds of the CONTRACT §5 table.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CoordinatorKind {
    /// Legacy-mail bridge (MX, DKIM egress, legacy client surfaces). The mail
    /// *adapter*; keep the name "gateway" for it only (STYLE §6).
    Gateway,
    /// Mesh reachability for NAT'd peers (Circuit Relay v2).
    Relay,
    /// Forwards SFrame-encrypted call/stream media; scales calls (RFC 9605).
    MediaRelay,
    /// ngrok-style public subdomains for arbitrary box services (REACH profile).
    ReachabilityAdapter,
    /// Managed infrastructure — the four DEPOT elementals `box`/`bucket`/`volume`/`edge-fn`, with
    /// `database`, `queue`, `cdn`, image `registry`, static `site` and hosted inference as
    /// **formulas** composing them (`profiles/cloud.md` §3.6–§3.7). Draft. Its descriptor `policy`
    /// blob is `kotva_depot::DepotServicePolicy`.
    ///
    /// This kind **absorbed the former `compute` kind**. CONTRACT §5's design note ("Why there is
    /// no separate `compute` kind") records the fold: invoking a model endpoint is "an `edge-fn`
    /// with `artifact-source = operator` on a class declaring `gpu-count` — an *attribute wearing
    /// a kind's clothes*", and two kinds both meaning "run code on your machine" left a client
    /// with nothing to disambiguate them by. `"compute"` is therefore **not** a wire kind and is a
    /// hard decode-time reject — see [`Self::from_wire_str`].
    InfraService,
    /// Search / discovery / global product-and-price view.
    Indexer,
    /// Moderation labels, opt-in, subscribable.
    Labeler,
    /// Real-time supply↔demand matching (rides, delivery).
    Matcher,
    /// Dispute resolution (staked jury).
    Arbiter,
    /// Physical-world / real-fact attestation (delivered? ride done?).
    Oracle,
    /// Holds the trade float for a trade window — the family's one load-bearing
    /// exception (CONTRACT §1, ESCROW §9–§10), disclosed not hidden.
    CustodialEscrow,
}

impl CoordinatorKind {
    /// The stable string id of the kind (as it appears in a descriptor).
    pub fn as_str(self) -> &'static str {
        match self {
            CoordinatorKind::Gateway => "gateway",
            CoordinatorKind::Relay => "relay",
            CoordinatorKind::MediaRelay => "media-relay",
            CoordinatorKind::ReachabilityAdapter => "reachability-adapter",
            CoordinatorKind::InfraService => "infra-service",
            CoordinatorKind::Indexer => "indexer",
            CoordinatorKind::Labeler => "labeler",
            CoordinatorKind::Matcher => "matcher",
            CoordinatorKind::Arbiter => "arbiter",
            CoordinatorKind::Oracle => "oracle",
            CoordinatorKind::CustodialEscrow => "custodial-escrow",
        }
    }

    /// The *typical* visibility from the CONTRACT §5 table. This is the default a
    /// well-behaved operator declares; the actual declaration is the operator's and
    /// is authoritative — a client checks the operator's declared value, never this
    /// table. `None` means the kind has no single default (e.g. the indexer, whose
    /// corpus is public plaintext but whose query channel varies).
    pub fn typical_visibility(self) -> Option<ContentVisibility> {
        use AssuranceLevel::*;
        use VisibilityClass::*;
        let v = |c, l| Some(ContentVisibility::new(c, l));
        match self {
            // Legacy leg is plaintext — a disclosed trust boundary.
            CoordinatorKind::Gateway => v(Terminating, Declared),
            CoordinatorKind::Relay => v(Blind, Structural),
            // Media payload sealed by SFrame; per-frame metadata/RTP routing visible.
            CoordinatorKind::MediaRelay => v(BlindRouting, Structural),
            // SNI-passthrough preferred; assurance is scoped by cert ownership
            // (REACH-1a) — structural for an own-domain name, declared for a bare
            // adapter-zone vanity. Structural is the RECOMMENDED profile.
            CoordinatorKind::ReachabilityAdapter => v(BlindRouting, Structural),
            // Corpus is public plaintext (nothing to be blind about); the query
            // channel is terminating unless attested — no single default.
            CoordinatorKind::Indexer => None,
            // Labels public objects — visibility is n/a to a delivery path.
            CoordinatorKind::Labeler => None,
            CoordinatorKind::Matcher => v(Terminating, Declared),
            // The operator's hardware sees whatever the workload holds — a disclosed trust
            // boundary. A TEE-attested variant (`blind`/`attested`) is an operator's declaration
            // to make, not this table's default; see `pier-infra-service`.
            CoordinatorKind::InfraService => v(Terminating, Declared),
            CoordinatorKind::Arbiter => v(Terminating, Declared),
            CoordinatorKind::Oracle => v(Terminating, Declared),
            CoordinatorKind::CustodialEscrow => v(Terminating, Declared),
        }
    }

    /// Whether this kind is the disclosed load-bearing exception — the only kind
    /// that does not fade once hired (CONTRACT §1, SEC-6/R-6). Everything else is
    /// hired-not-depended-on: removing it degrades reach, never function.
    pub fn is_load_bearing_exception(self) -> bool {
        matches!(self, CoordinatorKind::CustodialEscrow)
    }

    /// Parse the stable string id back into a kind (the inverse of [`Self::as_str`]), failing
    /// closed (`None`) on any unknown value — used decoding a wire descriptor (`descriptor.rs`),
    /// never guessing at an unrecognized kind string.
    ///
    /// `"compute"` is deliberately **absent**: CONTRACT §5 folded it into `infra-service`, and
    /// §18-wire-format §18.8a.1 requires a decoder reject it. A folded kind that still parses is
    /// precisely how a retired concept survives in the field, so this is a hard `None` and is
    /// pinned by `compute_is_a_hard_reject_on_the_wire`.
    pub fn from_wire_str(s: &str) -> Option<Self> {
        Some(match s {
            "gateway" => CoordinatorKind::Gateway,
            "relay" => CoordinatorKind::Relay,
            "media-relay" => CoordinatorKind::MediaRelay,
            "reachability-adapter" => CoordinatorKind::ReachabilityAdapter,
            "infra-service" => CoordinatorKind::InfraService,
            "indexer" => CoordinatorKind::Indexer,
            "labeler" => CoordinatorKind::Labeler,
            "matcher" => CoordinatorKind::Matcher,
            "arbiter" => CoordinatorKind::Arbiter,
            "oracle" => CoordinatorKind::Oracle,
            "custodial-escrow" => CoordinatorKind::CustodialEscrow,
            _ => return None,
        })
    }

    /// Whether this kind belongs to the disclosed scarce-network-reachability
    /// class (CONTRACT §2.3, THREAT-MODEL R-6) — a resource an ISP/host allocates,
    /// not something a user can always self-provision. Two members: the `gateway`
    /// (reputable IP + unblocked port 25) and the `reachability-adapter` (public
    /// reachable ingress).
    pub fn is_scarce_reachability(self) -> bool {
        matches!(
            self,
            CoordinatorKind::Gateway | CoordinatorKind::ReachabilityAdapter
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The CONTRACT §5 registry, in the same order as `kotva_coordinator::CoordinatorKind`.
    /// §5 is explicit that it is "the single canonical, authoritative list … **eleven** kinds …
    /// and no other document may enumerate a different count".
    const ALL_KINDS: [CoordinatorKind; 11] = [
        CoordinatorKind::Gateway,
        CoordinatorKind::Relay,
        CoordinatorKind::MediaRelay,
        CoordinatorKind::ReachabilityAdapter,
        CoordinatorKind::InfraService,
        CoordinatorKind::Indexer,
        CoordinatorKind::Labeler,
        CoordinatorKind::Matcher,
        CoordinatorKind::Arbiter,
        CoordinatorKind::Oracle,
        CoordinatorKind::CustodialEscrow,
    ];

    #[test]
    fn as_str_from_str_round_trips_every_kind() {
        for k in ALL_KINDS {
            assert_eq!(CoordinatorKind::from_wire_str(k.as_str()), Some(k));
        }
        assert_eq!(CoordinatorKind::from_wire_str("not-a-kind"), None);
    }

    /// The registry is the eleven of CONTRACT §5 and matches `kotva-coordinator`'s wire strings
    /// byte for byte. Asserted as an exact set, so neither a silently-added twelfth kind nor a
    /// dropped one can read as a pass.
    #[test]
    fn the_registry_is_exactly_the_canonical_eleven() {
        let wire: Vec<&str> = ALL_KINDS.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            wire,
            [
                "gateway",
                "relay",
                "media-relay",
                "reachability-adapter",
                "infra-service",
                "indexer",
                "labeler",
                "matcher",
                "arbiter",
                "oracle",
                "custodial-escrow",
            ],
            "the CONTRACT §5 registry, verbatim as kotva-coordinator emits it"
        );
    }

    /// CONTRACT §5 folded `compute` into `infra-service`; §18.8a.1 requires a decoder reject the
    /// retired string. This is the pier-side twin of kotva-coordinator's `DESCRIPTOR_KIND_COMPUTE`
    /// corruption control — without it, the two implementations disagree about what is a valid
    /// descriptor, which is the whole point of a shared registry.
    #[test]
    fn compute_is_a_hard_reject_on_the_wire() {
        assert_eq!(
            CoordinatorKind::from_wire_str("compute"),
            None,
            "`compute` is a retired kind (folded into `infra-service`) and MUST NOT decode"
        );
        assert!(
            !ALL_KINDS.iter().any(|k| k.as_str() == "compute"),
            "no kind may emit the retired `compute` string"
        );
    }

    #[test]
    fn relay_is_structurally_blind() {
        let v = CoordinatorKind::Relay.typical_visibility().unwrap();
        assert!(v.is_verifiably_blind());
    }

    #[test]
    fn gateway_is_terminating_and_scarce() {
        assert_eq!(
            CoordinatorKind::Gateway.typical_visibility().unwrap().class,
            VisibilityClass::Terminating
        );
        assert!(CoordinatorKind::Gateway.is_scarce_reachability());
    }

    #[test]
    fn only_custodial_escrow_is_load_bearing() {
        for k in [
            CoordinatorKind::Gateway,
            CoordinatorKind::Relay,
            CoordinatorKind::MediaRelay,
            CoordinatorKind::ReachabilityAdapter,
            CoordinatorKind::Matcher,
            CoordinatorKind::Oracle,
        ] {
            assert!(!k.is_load_bearing_exception(), "{} must fade", k.as_str());
        }
        assert!(CoordinatorKind::CustodialEscrow.is_load_bearing_exception());
    }

    #[test]
    fn exactly_two_scarce_reachability_members() {
        assert_eq!(ALL_KINDS.len(), 11, "the registry is the canonical eleven");
        let scarce: Vec<_> = ALL_KINDS
            .into_iter()
            .filter(|k| k.is_scarce_reachability())
            .collect();
        assert_eq!(scarce.len(), 2);
    }
}
