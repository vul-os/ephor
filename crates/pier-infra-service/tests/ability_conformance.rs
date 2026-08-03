//! `ability-conformance` (`profiles/cloud.md` §7) — kotva's probe, run against **pier's** §5.2 table.
//!
//! # Why this test is here and not in kotva
//!
//! §5.2 closes the ability registry so that one open-source client can drive any conformant gateway.
//! §7 turns that into a measurable claim. Both are worthless if the only thing that ever checks a
//! verb table is the crate that defines it — a round-trip inside one implementation re-reads
//! whatever it just wrote. This is the **first foreign consumer** of `kotva_depot::probe`: pier's
//! `control` module transcribes the §5.2 table from the profile document independently
//! (`crates/pier-infra-service/src/control.rs` re-exports nothing from `kotva-depot`), and the probe
//! is pointed at it.
//!
//! # What a pass here does and does not mean
//!
//! It means pier accepts all 44 §5.2 verb/elemental pairs and refuses all 236 near-misses — the
//! coinages (`terminate`, `delete-box`, `create`, `reboot`, `exec`, `ssh`), the case variants
//! (`DESTROY`, `Destroy`), the whitespace and empty strings, the prefix traps, and every verb that
//! belongs to a *different* elemental (`attach` on a `bucket`).
//!
//! It does **not** mean pier and any other gateway interoperate. §7 is explicit that below two
//! independent implementations the metric is vacuous and an aggregator MUST NOT read a pass as
//! evidence of interoperability; pier is one gateway, and the binding check in that state remains
//! the schema vector corpus. `kotva_depot::probe::VACUITY_CAVEAT` carries the sentence and this test
//! asserts it travels with the result.

use kotva_depot::probe::{
    self, expected_near_misses, expected_verbs, AbilityConformance, NEAR_MISS_COINAGES,
    VACUITY_CAVEAT,
};
use kotva_depot::{EvidenceKind, MeasurementValue, Method, Metric, Service};
use pier_infra_service::control;

/// Drive the probe against pier's own vocabulary gate.
fn run() -> AbilityConformance {
    probe::probe_all_elementals(|service: Service, ability: &str| {
        control::accepts_ability(service.as_str(), ability)
    })
}

#[test]
fn pier_speaks_the_section_5_2_vocabulary_without_coinage_or_aliasing() {
    let out = run();
    assert!(out.passed(), "{out}");

    // Coverage, asserted rather than assumed: a probe that examined nothing must not read as a
    // pass, so the counts are checked here too and not only inside `passed()`.
    assert_eq!(
        out.verbs_probed(),
        44,
        "4x7 common + 5 + 4 + 4 + 3 per-elemental"
    );
    assert_eq!(
        out.near_misses_probed(),
        NEAR_MISS_COINAGES.len() * 4 + 44,
        "the shared coinage corpus per elemental, plus each elemental's cross-elemental verbs"
    );
    assert!(NEAR_MISS_COINAGES.len() >= 50);

    for (report, service) in out.reports.iter().zip(Service::ALL) {
        assert_eq!(report.service, service);
        assert!(report.coverage_is_complete(), "{report}");
        assert_eq!(report.verbs_probed, expected_verbs(service));
        assert_eq!(report.near_misses_probed, expected_near_misses(service));
        assert!(report.refused_registry_verbs.is_empty(), "{report}");
        assert!(report.accepted_coinages.is_empty(), "{report}");
    }
}

#[test]
fn the_probe_is_pointed_at_pier_and_not_at_kotvas_own_registry() {
    // The tautology guard. If pier's table were a re-export of `kotva_depot::Ability`, this test
    // suite would prove nothing at all — so pin the two properties that make pier's answer its own:
    // it is reached through pier's public API, and it takes wire strings, not a parsed `Ability`.
    assert!(control::accepts_ability("box", "console"));
    assert!(!control::accepts_ability("bucket", "console"));
    // ...and the same answers arrive through the coordinator itself, which is what a real request
    // would traverse.
    use pier_economics::IdentityKey;
    let (coord, _signed) = pier_infra_service::InfraServiceCoordinator::signed(
        &IdentityKey::from_seed(&[9u8; 32]),
        pier_infra_service::InfraServiceChannel::Terminating,
        pier_economics::Cbor::empty(),
        None,
        false,
    );
    assert!(coord.accepts_ability("volume", "attach"));
    assert!(!coord.accepts_ability("volume", "terminate"));
}

#[test]
fn the_result_is_a_section_7_ability_conformance_measurement() {
    let out = run();
    let observed_at = 1_754_200_000_000;
    let ms = out.measurements(
        observed_at,
        Some((
            EvidenceKind::Recipe,
            "cargo test -p pier-infra-service --test ability_conformance".to_string(),
        )),
    );
    assert_eq!(
        ms.len(),
        4,
        "§7 carries one `service` per claim, so four claims"
    );
    for (m, service) in ms.iter().zip(Service::ALL) {
        assert_eq!(m.service, service);
        assert_eq!(m.metric, Metric::AbilityConformance);
        assert_eq!(m.method, Method::Probe);
        assert_eq!(m.value, MeasurementValue::Bool(true));
        assert_eq!(m.observed_at, observed_at);
        assert!(m.is_wellformed(), "§7 types ability-conformance as a bool");
        // §7: a consumer SHOULD re-run a reproducible probe rather than trust the reported value.
        assert!(m.is_independently_checkable());
    }
}

#[test]
fn the_vacuity_caveat_travels_with_the_pass() {
    // §7, normative for aggregators: with one implementation this metric is vacuous. A green result
    // from this repo alone is not evidence of interoperability, and it must not be quotable as if it
    // were.
    let out = run();
    assert!(out.passed());
    let rendered = out.to_string();
    assert!(rendered.contains(VACUITY_CAVEAT), "{rendered}");
    assert!(VACUITY_CAVEAT.contains("VACUOUS"));
    assert!(VACUITY_CAVEAT.contains("MUST NOT"));
}

#[test]
fn the_probe_would_notice_if_pier_started_aliasing() {
    // The false-negative control for THIS test file, not for pier: it proves the harness above is
    // wired to something that can fail. A stand-in oracle that accepts `terminate` — the exact
    // coinage §5.2 names — must go red, and one that refuses `export` must also go red. Without
    // both, a passing suite is consistent with a probe that never looked.
    let aliasing = probe::probe_all_elementals(|service: Service, ability: &str| {
        ability == "terminate" || control::accepts_ability(service.as_str(), ability)
    });
    assert!(!aliasing.passed());
    assert!(aliasing.reports[0]
        .accepted_coinages
        .contains(&"terminate".to_string()));

    let under_implementing = probe::probe_all_elementals(|service: Service, ability: &str| {
        ability != "export" && control::accepts_ability(service.as_str(), ability)
    });
    assert!(!under_implementing.passed());
    assert!(under_implementing.reports[0]
        .refused_registry_verbs
        .contains(&"export"));
}
