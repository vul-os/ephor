//! # infra-service — the `infra-service` coordinator kind, *draft* (CONTRACT §5)
//!
//! An **infra-service** coordinator provides managed infrastructure: the four DEPOT elementals
//! `box`/`bucket`/`volume`/`edge-fn`, with `database`, `queue`, `cdn`, image `registry`, static
//! `site` and hosted inference as **formulas** composing them (`profiles/cloud.md` §3.6–§3.7).
//! CONTRACT §5 marks this kind **draft** and defers it out of Core v1, and this crate keeps that
//! disclosure: the kind is real (`CoordinatorKind::InfraService` exists and is checkable), but its
//! shape is the least settled of the eleven in the table.
//!
//! ## This kind absorbed the former `compute` kind
//!
//! There is no separate `compute` coordinator. CONTRACT §5's design note ("Why there is no
//! separate `compute` kind") records the fold: invoking a model endpoint is "an `edge-fn` with
//! `artifact-source = operator` on a class declaring `gpu-count` — an **attribute wearing a kind's
//! clothes**", and two kinds both meaning "run code on your machine" left a client with nothing to
//! disambiguate them by. Hosted/outsourced computation — private-AI inference on rented GPU — is
//! therefore served here, as a formula, not as a kind of its own. `"compute"` is not a wire kind
//! and `CoordinatorKind::from_wire_str` rejects it outright.
//!
//! ## Visibility: `terminating` default, `attested` (TEE) for blind workloads
//!
//! By default the operator's hardware sees the plaintext input/output of the workload it runs — a
//! disclosed trust boundary, same shape as the `gateway`/`matcher`/`arbiter`/`oracle` default. The
//! alternative is a **blind workload**: it runs inside a TEE that attests the operator holds no
//! readable copy of the input/output, at the cost of trading operator-trust for chip-vendor-trust.
//! CONTRACT §3.4 / THREAT-MODEL R-4's honest disclosure applies here in full: attestation is
//! hardware-trust, not the structural absence of a key, and a side-channel history exists. This
//! crate documents the option honestly via [`InfraServiceChannel::Attested`] but implements **no
//! TEE integration**.
//!
//! ## Not a delivery path — `Gate::NoDeliveryPath`
//!
//! An infra-service coordinator runs a workload for the party that provisioned it; it does not
//! classify, rank, or gate content on any delivery/authoritative path, nor rank an opt-in
//! corpus/pool the way `indexer`/`labeler`/`matcher` do. [`Gate::NoDeliveryPath`] is the honest
//! answer — see [`InfraServiceCoordinator::delivery_path_gate`].
//!
//! ## Scaffold disclosure
//!
//! This crate fixes the CONTRACT posture and a signed descriptor — it is **not** a working
//! infrastructure service. Provisioning, execution, result delivery, and any TEE attestation
//! integration are all future work; nothing here runs a real workload yet. In particular it does
//! **not** yet build a `kotva_depot::DepotServicePolicy` policy blob — the `policy` field is an
//! opaque [`Cbor`] the caller supplies.
//!
//! ## The one thing here that *is* implemented: the §5.2 control vocabulary
//!
//! [`control`] is pier's own transcription of `profiles/cloud.md` §5.2 — the closed ability registry
//! — and its fail-closed gate. It is written out longhand rather than re-exported from `kotva-depot`
//! **on purpose**: §5.2 is closed so that one open-source client can drive any conformant gateway,
//! and that guarantee is only worth something once two implementations of the table exist and are
//! checked against each other. `tests/ability_conformance.rs` runs kotva's `ability-conformance`
//! probe (§7) against this table, in both directions — every registry verb accepted, every near-miss
//! coinage refused.

#![forbid(unsafe_code)]

pub mod control;

use pier_conformance::{Coordinator, Gate, LockIn, Metering, SelfHost, Settlement};
use pier_economics::descriptor::{Descriptor, SignedDescriptor, Tariff};
use pier_economics::kinds::CoordinatorKind;
use pier_economics::visibility::{AssuranceLevel, ContentVisibility, VisibilityClass};
use pier_economics::{Cbor, IdentityKey};

/// The visibility an infra-service coordinator declares over the workload it runs (CONTRACT §5:
/// `terminating` default / `attested` via TEE, for a blind workload).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InfraServiceChannel {
    /// The declared **default** (CONTRACT §5): the operator's hardware sees the job's plaintext
    /// input/output to run it — a disclosed trust boundary, not a silent one.
    Terminating,
    /// The disclosed **alternative** — "blind workload": it runs inside a TEE that attests it
    /// holds no readable copy of the input/output. Honestly trades operator-trust for
    /// chip-vendor-trust (§3.4, THREAT-MODEL R-4) — documented here as an option; **no TEE
    /// integration exists in this scaffold**.
    Attested,
}

impl InfraServiceChannel {
    /// The [`ContentVisibility`] a conformant infra-service descriptor MUST carry for this channel
    /// choice (COORD-4/COORD-5).
    pub fn declared_visibility(self) -> ContentVisibility {
        match self {
            InfraServiceChannel::Terminating => {
                ContentVisibility::new(VisibilityClass::Terminating, AssuranceLevel::Declared)
            }
            // Blind workload: the operator is architecturally unable to read it, so the class
            // shifts to `Blind`; the level is `Attested` because that guarantee rests on hardware
            // attestation rather than the structural absence of a key.
            InfraServiceChannel::Attested => {
                ContentVisibility::new(VisibilityClass::Blind, AssuranceLevel::Attested)
            }
        }
    }
}

/// An `infra-service` coordinator's posture for `pier_conformance::check` (CONTRACT §2/§4/§5/§6).
/// Kept explicitly *draft* per CONTRACT §5's own table — see the crate docs.
pub struct InfraServiceCoordinator {
    descriptor: Descriptor,
    /// Whether this coordinator meters workloads and issues signed receipts (CONTRACT
    /// §6/COORD-7).
    metered: bool,
}

impl InfraServiceCoordinator {
    /// Wrap an already-built `InfraService`-kind [`Descriptor`]. Does not itself validate
    /// `descriptor.kind`/`descriptor.visibility` — a mismatched descriptor surfaces as a
    /// `pier_conformance::check` finding. Prefer [`InfraServiceCoordinator::signed`] for the common
    /// case of minting a fresh, correctly-shaped descriptor.
    pub fn new(descriptor: Descriptor, metered: bool) -> Self {
        Self {
            descriptor,
            metered,
        }
    }

    /// Build **and sign** a fresh, correctly-shaped `infra-service` descriptor from a real `kotva-core`
    /// identity (CONTRACT §2.1) declaring `channel`'s visibility.
    pub fn signed(
        ik: &IdentityKey,
        channel: InfraServiceChannel,
        policy: Cbor,
        tariff: Option<Tariff>,
        metered: bool,
    ) -> (Self, SignedDescriptor) {
        let descriptor = Descriptor {
            identity: ik.public(),
            kind: CoordinatorKind::InfraService,
            visibility: channel.declared_visibility(),
            policy,
            tariff,
        };
        let signed = descriptor.sign(ik);
        (Self::new(descriptor, metered), signed)
    }

    /// Whether this coordinator recognises `ability` as a §5.2 verb for the `service` elemental.
    ///
    /// The vocabulary gate every control request passes first — see [`control`]. Unknown verbs are
    /// refused, never mapped onto a near-match, and a verb is scoped to its elemental. This is the
    /// coordinator-level entry point the `ability-conformance` probe (§7) drives.
    pub fn accepts_ability(&self, service: &str, ability: &str) -> bool {
        control::accepts_ability(service, ability)
    }
}

impl Coordinator for InfraServiceCoordinator {
    fn kind(&self) -> CoordinatorKind {
        CoordinatorKind::InfraService
    }

    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }

    fn lock_in(&self) -> LockIn {
        // CONTRACT §2.2: provisioning a future workload with a different operator is a config
        // change — no ongoing identity, keys, or data custody lives with an infra-service coordinator
        // between jobs.
        LockIn::None
    }

    fn self_host(&self) -> SelfHost {
        // Not a member of the disclosed scarce-reachability exception class (CONTRACT §2.3) —
        // anyone who can rent or own the hardware can run their own infra-service coordinator.
        SelfHost::Backstop
    }

    fn delivery_path_gate(&self) -> Gate {
        // Outsourced computation sits on no §4 delivery/authoritative content path, and is not a
        // ranking of an opt-in derived view either. See the crate docs' dedicated section.
        Gate::NoDeliveryPath
    }

    fn metering(&self) -> Metering {
        if self.metered {
            Metering::SignedReceiptsToPayer
        } else {
            Metering::NotMetered
        }
    }

    fn settlement(&self) -> Settlement {
        // DIRECTION §5: no protocol token, ever.
        Settlement::ExistingAssetsOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pier_conformance::check;

    fn ik(seed: u8) -> IdentityKey {
        IdentityKey::from_seed(&[seed; 32])
    }

    #[test]
    fn signed_infra_service_descriptor_verifies_and_declares_terminating_by_default() {
        let (_coord, signed) = InfraServiceCoordinator::signed(
            &ik(1),
            InfraServiceChannel::Terminating,
            Cbor::empty(),
            None,
            false,
        );
        assert!(
            signed.verify().is_ok(),
            "a real kotva-core signature must verify"
        );
        // The wire string is `infra-service`, never the retired `compute` this kind absorbed.
        // kotva-coordinator's frozen DESCRIPTOR_V0 carries `02 6d "infra-service"` and its
        // DESCRIPTOR_KIND_COMPUTE corruption control requires `"compute"` be rejected; this
        // assertion is the pier side of that agreement.
        assert_eq!(signed.descriptor.kind.as_str(), "infra-service");
        assert_ne!(signed.descriptor.kind.as_str(), "compute");
        assert_eq!(
            signed.descriptor.visibility.class,
            VisibilityClass::Terminating
        );
        assert_eq!(signed.descriptor.visibility.level, AssuranceLevel::Declared);
    }

    #[test]
    fn attested_blind_workload_declares_blind_attested() {
        let (_coord, signed) = InfraServiceCoordinator::signed(
            &ik(2),
            InfraServiceChannel::Attested,
            Cbor::empty(),
            None,
            false,
        );
        assert_eq!(signed.descriptor.visibility.class, VisibilityClass::Blind);
        assert_eq!(signed.descriptor.visibility.level, AssuranceLevel::Attested);
        assert!(signed.descriptor.visibility.is_verifiably_blind());
    }

    #[test]
    fn a_free_infra_service_coordinator_is_fully_conformant() {
        let (coord, _signed) = InfraServiceCoordinator::signed(
            &ik(3),
            InfraServiceChannel::Terminating,
            Cbor::empty(),
            None,
            false,
        );
        let report = check(&coord);
        assert!(report.is_conformant(), "{:?}", report.findings);
    }

    #[test]
    fn a_metered_infra_service_coordinator_is_also_conformant() {
        let (coord, _signed) = InfraServiceCoordinator::signed(
            &ik(4),
            InfraServiceChannel::Terminating,
            Cbor::empty(),
            None,
            true,
        );
        let report = check(&coord);
        assert!(report.is_conformant(), "{:?}", report.findings);
        assert!(matches!(coord.metering(), Metering::SignedReceiptsToPayer));
    }

    #[test]
    fn infra_service_has_no_delivery_path_to_gate() {
        let (coord, _signed) = InfraServiceCoordinator::signed(
            &ik(5),
            InfraServiceChannel::Terminating,
            Cbor::empty(),
            None,
            false,
        );
        assert!(matches!(coord.delivery_path_gate(), Gate::NoDeliveryPath));
    }

    #[test]
    fn infra_service_is_not_the_scarce_reachability_exception() {
        assert!(!CoordinatorKind::InfraService.is_scarce_reachability());
        let (coord, _signed) = InfraServiceCoordinator::signed(
            &ik(6),
            InfraServiceChannel::Terminating,
            Cbor::empty(),
            None,
            false,
        );
        assert!(matches!(coord.self_host(), SelfHost::Backstop));
    }

    #[test]
    fn infra_service_mints_no_token() {
        let (coord, _signed) = InfraServiceCoordinator::signed(
            &ik(7),
            InfraServiceChannel::Terminating,
            Cbor::empty(),
            None,
            false,
        );
        assert!(matches!(coord.settlement(), Settlement::ExistingAssetsOnly));
    }

    #[test]
    fn wrong_kind_descriptor_is_a_coord1_violation() {
        let key = ik(8);
        let descriptor = Descriptor {
            identity: key.public(),
            kind: CoordinatorKind::Relay,
            visibility: InfraServiceChannel::Terminating.declared_visibility(),
            policy: Cbor::empty(),
            tariff: None,
        };
        let coord = InfraServiceCoordinator::new(descriptor, false);
        let report = check(&coord);
        assert!(!report.is_conformant());
    }
}
