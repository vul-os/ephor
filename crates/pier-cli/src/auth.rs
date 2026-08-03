//! Authorisation — a `CapabilityToken` (§18.7.3), and **never** a bearer API key.
//!
//! DEPOT-11 makes the control plane a capability rather than an API key, and §5.3 states the
//! consequence bluntly: *"Authorisation is the token, never the transport. Possession of a network
//! path grants nothing."* This module is where that becomes code, and where it **fails closed**.
//!
//! Three refusals, in the order they bite:
//!
//! 1. **No token, no call.** There is no anonymous path and no fallback. If a token cannot be
//!    found, decoded, and verified, the command refuses — it does not "try unauthenticated and see".
//! 2. **No coordinator, no call.** A DEPOT resource string names no operator: `depot:box/7f3a91c2`
//!    at two gateways is the same string denoting different machines. So §5.1 requires every DEPOT
//!    capability to carry a `depot:coordinator` caveat naming the intended coordinator's `IK`, and
//!    requires that presence be *checked* — §18.7.3 fails closed on an *unrecognised* caveat, but
//!    an **absent** one is well-formed and valid everywhere, which is exactly the confused-deputy
//!    hole. `pier` therefore refuses to send a token that names no coordinator, and refuses to send
//!    one naming a different coordinator than `--coordinator`.
//! 3. **No unknown verbs.** Every `resource` and `ability` in the token is parsed through
//!    [`kotva_depot`]'s closed registries on load. A token carrying `terminate` is rejected here,
//!    not silently mapped onto `destroy`.
//!
//! What is **not** built: minting. `pier auth token` cannot issue a token, because there is no
//! keystore holding an issuer `IdentityKey` and no parent token to attenuate from. That is a
//! refusal, not a gap papered over with an API key.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kotva_core::capability::CapabilityToken;
use kotva_core::cbor::Cv;
use kotva_depot::{check_coordinator_binding, Ability, ResourceRef, CAVEAT_COORDINATOR};

/// The environment variable naming a file that holds a det-CBOR `CapabilityToken`.
pub const ENV_TOKEN: &str = "PIER_TOKEN";
/// The environment variable holding the target coordinator's `IK`, hex-encoded.
pub const ENV_COORDINATOR: &str = "PIER_COORDINATOR";

/// Every way authorisation refuses. There is no `Ok`-ish variant and no "degraded" mode: each of
/// these stops the command.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AuthError {
    /// No token was supplied at all.
    #[error(
        "no CapabilityToken. Pass --token <file> or set {ENV_TOKEN}=<file> (a det-CBOR \
         §18.7.3 token).\n\
         REFUSING to continue: DEPOT-11/§5.3 make authorisation the token, never the transport, so \
         there is no unauthenticated path to fall back to and `pier` will not invent one."
    )]
    NoToken,

    /// The token file could not be read.
    #[error("cannot read token file {path}: {source}")]
    Read {
        /// The path that failed.
        path: PathBuf,
        /// The underlying I/O failure.
        source: std::io::Error,
    },

    /// The bytes are not a well-formed §18.7.3 token.
    #[error("token is not a well-formed §18.7.3 CapabilityToken: {0}")]
    Decode(kotva_core::cbor::CborError),

    /// The signature does not verify under `iss`.
    #[error("token signature does not verify under its issuer key (§18.9.14): {0}")]
    BadSignature(kotva_core::identity::IdentityError),

    /// Outside the token's validity window.
    #[error("token is not valid now: nbf={nbf} exp={exp} now={now} (ms epoch)")]
    OutOfWindow {
        /// Not-before, ms epoch.
        nbf: u64,
        /// Expiry, ms epoch.
        exp: u64,
        /// Wall clock at check time, ms epoch.
        now: u64,
    },

    /// No target coordinator was supplied, so the §5.1 binding cannot be checked.
    #[error(
        "no target coordinator. Pass --coordinator <hex IK> or set {ENV_COORDINATOR}.\n\
         REFUSING to continue: §5.1 requires every DEPOT capability to be bound to the coordinator \
         it is meant for, and a binding cannot be checked against a target that was never named — \
         sending the token anyway is the confused-deputy case the caveat exists to close."
    )]
    NoCoordinator,

    /// `--coordinator` was not 32 bytes of hex.
    #[error("--coordinator must be a hex-encoded 32-byte Ed25519 IK ({0})")]
    BadCoordinator(String),

    /// A capability in the token names a resource outside the §5.1 grammar, or an ability outside
    /// the closed §5.2 registry.
    #[error("token carries a capability this client refuses to use: {0}")]
    UnusableCapability(kotva_depot::DepotError),

    /// No capability in the token is bound to the named coordinator.
    #[error(
        "token is not bound to coordinator {coordinator}: the `{CAVEAT_COORDINATOR}` caveat is \
         absent from every capability, or names another key (§5.1). A token that names no \
         coordinator is valid at every DEPOT gateway — that is the defect, not a convenience."
    )]
    NotBound {
        /// The hex `IK` that was being targeted.
        coordinator: String,
    },

    /// The token is valid and bound, but grants nothing covering this request.
    #[error(
        "token does not authorise `{ability}` on `{resource}`.\n\
         It grants: {granted}"
    )]
    NotAuthorised {
        /// The §5.1 resource the command needed.
        resource: String,
        /// The §5.2 ability the command needed.
        ability: &'static str,
        /// What the token does grant, rendered.
        granted: String,
    },
}

/// One capability from the token, with its resource and ability already parsed through the closed
/// registries — so an unusable grant is rejected at load, not at use.
#[derive(Debug, Clone)]
pub struct Grant {
    /// The §5.1 scope.
    pub resource: ResourceRef,
    /// The §5.2 verb.
    pub ability: Ability,
    /// The `depot:coordinator` caveat value, if the capability carries one.
    pub coordinator: Option<Vec<u8>>,
}

/// A loaded, signature-verified, in-window token whose grants all parse.
///
/// Constructing one of these is the only way to get past the auth gate; there is no variant that
/// represents "no token".
#[derive(Debug, Clone)]
pub struct Authorisation {
    /// The verified token.
    pub token: CapabilityToken,
    /// Its parsed grants.
    pub grants: Vec<Grant>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Pull the `depot:coordinator` caveat out of a capability's caveat map.
fn coordinator_caveat(caveats: Option<&Cv>) -> Option<Vec<u8>> {
    match caveats {
        Some(Cv::TextMap(entries)) => entries.iter().find_map(|(k, v)| {
            if k == CAVEAT_COORDINATOR {
                match v {
                    Cv::Bytes(b) => Some(b.clone()),
                    _ => None,
                }
            } else {
                None
            }
        }),
        _ => None,
    }
}

/// Decode a hex-encoded coordinator `IK`, insisting on 32 bytes.
pub fn parse_coordinator(hex_ik: &str) -> Result<Vec<u8>, AuthError> {
    let raw = hex::decode(hex_ik.trim())
        .map_err(|e| AuthError::BadCoordinator(format!("not hex: {e}")))?;
    if raw.len() != 32 {
        return Err(AuthError::BadCoordinator(format!(
            "got {} bytes, want 32",
            raw.len()
        )));
    }
    Ok(raw)
}

/// Resolve where the token comes from: `--token`, else `$PIER_TOKEN`. There is deliberately no
/// implicit well-known path — a credential picked up from a location the user did not name is a
/// credential they did not know they were presenting.
pub fn token_path(flag: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = flag {
        return Some(p.to_path_buf());
    }
    std::env::var_os(ENV_TOKEN).map(PathBuf::from)
}

/// Load, decode, and verify the token — signature, validity window, and every grant through the
/// closed registries. Does **not** yet check the coordinator binding or the requested scope; see
/// [`Authorisation::authorise`].
pub fn load(flag: Option<&Path>) -> Result<Authorisation, AuthError> {
    let path = token_path(flag).ok_or(AuthError::NoToken)?;
    let bytes = std::fs::read(&path).map_err(|source| AuthError::Read { path, source })?;
    from_bytes(&bytes, now_ms())
}

/// The testable core of [`load`]: everything except reading the file.
pub fn from_bytes(bytes: &[u8], now: u64) -> Result<Authorisation, AuthError> {
    let token = CapabilityToken::from_det_cbor(bytes).map_err(AuthError::Decode)?;
    token.verify().map_err(AuthError::BadSignature)?;
    if now < token.nbf || now >= token.exp {
        return Err(AuthError::OutOfWindow {
            nbf: token.nbf,
            exp: token.exp,
            now,
        });
    }
    let mut grants = Vec::with_capacity(token.caps.len());
    for cap in &token.caps {
        let resource = ResourceRef::parse(&cap.resource).map_err(AuthError::UnusableCapability)?;
        // Fail closed on a verb outside the closed registry — never map it onto a near-match.
        let ability = Ability::from_str(&cap.ability).ok_or_else(|| {
            AuthError::UnusableCapability(kotva_depot::DepotError::UnknownRegistryValue {
                registry: "ability",
                value: cap.ability.clone(),
            })
        })?;
        grants.push(Grant {
            resource,
            ability,
            coordinator: coordinator_caveat(cap.caveats.as_ref()),
        });
    }
    Ok(Authorisation { token, grants })
}

impl Authorisation {
    /// Check that this token authorises `ability` on `resource` **at** `coordinator_ik`.
    ///
    /// Both halves are load-bearing and both fail closed:
    ///
    /// * the §5.1 coordinator binding, via [`kotva_depot::check_coordinator_binding`] — a grant
    ///   whose caveat is absent or names another key does not count toward the decision at all;
    /// * the §5.1 attenuation predicate, via [`ResourceRef::covers`] — a grant over
    ///   `depot:box/*` covers `depot:box/7f3a`, and never the reverse.
    pub fn authorise(
        &self,
        coordinator_ik: &[u8],
        resource: &ResourceRef,
        ability: Ability,
    ) -> Result<(), AuthError> {
        let bound: Vec<&Grant> = self
            .grants
            .iter()
            .filter(|g| check_coordinator_binding(g.coordinator.as_deref(), coordinator_ik).is_ok())
            .collect();
        if bound.is_empty() {
            return Err(AuthError::NotBound {
                coordinator: hex::encode(coordinator_ik),
            });
        }
        if bound
            .iter()
            .any(|g| g.ability == ability && g.resource.covers(resource))
        {
            return Ok(());
        }
        Err(AuthError::NotAuthorised {
            resource: resource.to_wire(),
            ability: ability.as_str(),
            granted: render_grants(&bound),
        })
    }

    /// Render every grant, for `pier auth whoami`.
    pub fn render(&self) -> String {
        render_grants(&self.grants.iter().collect::<Vec<_>>())
    }
}

fn render_grants(grants: &[&Grant]) -> String {
    if grants.is_empty() {
        return "(nothing)".to_string();
    }
    grants
        .iter()
        .map(|g| {
            let bind = match &g.coordinator {
                Some(ik) => format!("@{}", hex::encode(ik)),
                None => "@UNBOUND (refused: §5.1)".to_string(),
            };
            format!("{} {} {}", g.ability.as_str(), g.resource.to_wire(), bind)
        })
        .collect::<Vec<_>>()
        .join("\n             ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kotva_core::capability::Capability;
    use kotva_core::identity::IdentityKey;

    fn caveats_for(ik: &[u8]) -> Option<Cv> {
        Some(Cv::TextMap(vec![(
            CAVEAT_COORDINATOR.to_string(),
            Cv::Bytes(ik.to_vec()),
        )]))
    }

    fn token_with(caps: Vec<Capability>) -> (Vec<u8>, IdentityKey) {
        let k = IdentityKey::from_seed(&[7u8; 32]);
        let t = CapabilityToken::issue(
            &k,
            vec![9u8; 32],
            caps,
            1_000,
            9_000_000_000_000,
            vec![1, 2, 3],
            None,
        );
        (t.det_cbor(), k)
    }

    fn coord() -> Vec<u8> {
        vec![0xABu8; 32]
    }

    #[test]
    fn a_bound_token_authorises_within_its_scope_and_not_outside_it() {
        let ik = coord();
        let (bytes, _) = token_with(vec![Capability {
            resource: "depot:box/*".into(),
            ability: "destroy".into(),
            caveats: caveats_for(&ik),
        }]);
        let a = from_bytes(&bytes, 2_000).unwrap();

        let one = ResourceRef::parse("depot:box/7f3a").unwrap();
        a.authorise(&ik, &one, Ability::Destroy).unwrap();

        // Wrong verb: no aliasing, no "close enough".
        assert!(a.authorise(&ik, &one, Ability::Console).is_err());
        // Wrong elemental.
        let vol = ResourceRef::parse("depot:volume/7f3a").unwrap();
        assert!(a.authorise(&ik, &vol, Ability::Destroy).is_err());
        // Broader than the grant — the direction that silently widens tokens if inverted.
        let all = ResourceRef::parse("depot:*").unwrap();
        assert!(a.authorise(&ik, &all, Ability::Destroy).is_err());
    }

    #[test]
    fn an_unbound_token_is_refused_even_though_it_grants_the_verb() {
        // The confused-deputy case: everything about this token is fine except that it names no
        // coordinator, so it is presentable at every DEPOT gateway.
        let (bytes, _) = token_with(vec![Capability {
            resource: "depot:box/*".into(),
            ability: "destroy".into(),
            caveats: None,
        }]);
        let a = from_bytes(&bytes, 2_000).unwrap();
        let one = ResourceRef::parse("depot:box/7f3a").unwrap();
        let err = a.authorise(&coord(), &one, Ability::Destroy).unwrap_err();
        assert!(matches!(err, AuthError::NotBound { .. }), "{err}");
    }

    #[test]
    fn a_token_bound_to_another_coordinator_is_refused() {
        let other = vec![0xCDu8; 32];
        let (bytes, _) = token_with(vec![Capability {
            resource: "depot:box/*".into(),
            ability: "destroy".into(),
            caveats: caveats_for(&other),
        }]);
        let a = from_bytes(&bytes, 2_000).unwrap();
        let one = ResourceRef::parse("depot:box/7f3a").unwrap();
        assert!(matches!(
            a.authorise(&coord(), &one, Ability::Destroy).unwrap_err(),
            AuthError::NotBound { .. }
        ));
    }

    #[test]
    fn a_coined_verb_in_the_token_is_rejected_at_load() {
        let ik = coord();
        let (bytes, _) = token_with(vec![Capability {
            resource: "depot:box/*".into(),
            ability: "terminate".into(),
            caveats: caveats_for(&ik),
        }]);
        let err = from_bytes(&bytes, 2_000).unwrap_err();
        assert!(matches!(err, AuthError::UnusableCapability(_)), "{err}");
        assert!(err.to_string().contains("terminate"), "{err}");
    }

    #[test]
    fn a_tampered_signature_is_refused() {
        let ik = coord();
        let (mut bytes, _) = token_with(vec![Capability {
            resource: "depot:box/*".into(),
            ability: "destroy".into(),
            caveats: caveats_for(&ik),
        }]);
        // Flip a byte in the middle of the encoded token.
        let mid = bytes.len() / 2;
        bytes[mid] ^= 0xFF;
        // Either the CBOR no longer decodes or the signature no longer verifies — both are
        // refusals, and neither yields an Authorisation.
        assert!(from_bytes(&bytes, 2_000).is_err());
    }

    #[test]
    fn an_expired_token_is_refused() {
        let ik = coord();
        let k = IdentityKey::from_seed(&[7u8; 32]);
        let t = CapabilityToken::issue(
            &k,
            vec![9u8; 32],
            vec![Capability {
                resource: "depot:box/*".into(),
                ability: "destroy".into(),
                caveats: caveats_for(&ik),
            }],
            1_000,
            2_000,
            vec![1],
            None,
        );
        let bytes = t.det_cbor();
        from_bytes(&bytes, 1_500).unwrap();
        assert!(matches!(
            from_bytes(&bytes, 2_000).unwrap_err(),
            AuthError::OutOfWindow { .. }
        ));
        assert!(matches!(
            from_bytes(&bytes, 500).unwrap_err(),
            AuthError::OutOfWindow { .. }
        ));
    }

    #[test]
    fn missing_token_is_a_refusal_not_a_default() {
        // No flag and no env var must not resolve to some well-known path.
        std::env::remove_var(ENV_TOKEN);
        assert!(token_path(None).is_none());
        assert!(matches!(load(None).unwrap_err(), AuthError::NoToken));
    }

    #[test]
    fn coordinator_must_be_32_bytes_of_hex() {
        assert!(parse_coordinator(&hex::encode([1u8; 32])).is_ok());
        assert!(parse_coordinator("zz").is_err());
        assert!(parse_coordinator(&hex::encode([1u8; 31])).is_err());
        assert!(parse_coordinator("").is_err());
    }
}
