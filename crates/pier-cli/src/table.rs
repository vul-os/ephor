//! **The command table** — the single source of truth for what `pier` can be asked to do.
//!
//! Every row names a CLI path (`box destroy`) and the [`Action`] it resolves to. There is
//! deliberately **no CLI-local verb vocabulary**: a DEPOT row carries a
//! [`kotva_depot::Ability`] value, not a string, so `pier box destroy` can only ever put
//! `destroy` on the wire. The §5.2 registry is closed to operator coinage, and a client that
//! invents `terminate` is the exact failure that rule exists to prevent — so the mapping is a
//! type, and `tests/ability_mapping.rs` walks the *built clap tree* back through this table
//! (both directions, with counts) so a row that stops being reachable, or a subcommand with no
//! row, fails the build.
//!
//! The friendly CLI noun stays flyctl-shaped where flyctl's is better known — `create`, `show`,
//! `put`, `get`, `rm` — because those are the words users already have in their fingers. The
//! *wire* verb underneath is `provision`, `inspect`, `write`, `read`, `delete`. `--help` prints
//! both, so the translation is visible rather than folklore.

use kotva_depot::{Ability, Service};

/// What a command does when it runs — and, for the DEPOT rows, the exact §5.2 verb it puts on
/// the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// A DEPOT control-plane call.
    ///
    /// `service` is `None` where the elemental is chosen at runtime rather than by the command
    /// itself (`pier logs --service`, `pier status` over `depot:*`); the ability is fixed either
    /// way, which is the property the mapping test checks.
    Depot {
        /// The §3 elemental this verb is scoped to, when the command fixes one.
        service: Option<Service>,
        /// The §5.2 ability. A value, never a string — see the module docs.
        ability: Ability,
    },
    /// Runs entirely on this machine: reads a file, derives a record, prints a table. Contacts no
    /// coordinator and therefore has no ability.
    ///
    /// This variant is the escape hatch the mapping test exists to fence in: it is enumerated
    /// exactly (`LOCAL_ACTIONS`) and the test asserts the set has not grown, so "it's local"
    /// cannot become a way to smuggle in a CLI-local verb.
    Local,
}

/// How much of a command is actually built.
///
/// Two levels, and the missing third is the point: nothing here is end-to-end real, because there
/// is no transport. A status is never upgraded by a command that merely *prints plausibly*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// **Performs no operation.** Resolves its resource and ability, checks authorisation, then
    /// refuses with a non-zero exit and says what is missing. It does not contact a coordinator
    /// and it never reports success.
    Scaffold,
    /// **Real, but local.** Fully implemented and does what it says — and what it says never
    /// includes contacting a coordinator.
    LocalOnly,
}

impl Status {
    /// The tag that appears in `--help`, so the status is visible without running anything.
    pub fn tag(self) -> &'static str {
        match self {
            Status::Scaffold => "[SCAFFOLD: sends nothing]",
            Status::LocalOnly => "[LOCAL-ONLY: contacts no coordinator]",
        }
    }
}

/// One row of the command table.
#[derive(Debug, Clone, Copy)]
pub struct Cmd {
    /// The CLI path, e.g. `["box", "destroy"]`. Length 1 or 2.
    pub path: &'static [&'static str],
    /// One-line `--help` summary, without the status or ability tags (those are appended).
    pub about: &'static str,
    /// What it resolves to.
    pub action: Action,
    /// How much of it is built.
    pub status: Status,
    /// What is *not* implemented, printed verbatim when the command runs. Empty for
    /// [`Status::LocalOnly`] rows that are complete.
    pub missing: &'static str,
}

impl Cmd {
    /// `"box destroy"` — the joined path, used as the table key.
    pub fn key(&self) -> String {
        self.path.join(" ")
    }

    /// The §5.2 ability this row puts on the wire, if it is a DEPOT row.
    pub fn ability(&self) -> Option<Ability> {
        match self.action {
            Action::Depot { ability, .. } => Some(ability),
            Action::Local => None,
        }
    }

    /// The §3 elemental this row is scoped to, if the command fixes one.
    pub fn service(&self) -> Option<Service> {
        match self.action {
            Action::Depot { service, .. } => service,
            Action::Local => None,
        }
    }

    /// The full `--help` `about` line: summary, status tag, and the wire verb.
    pub fn help_line(&self) -> String {
        match self.ability() {
            Some(a) => format!(
                "{} {} [ability: {}]",
                self.about,
                self.status.tag(),
                a.as_str()
            ),
            None => format!("{} {}", self.about, self.status.tag()),
        }
    }
}

const NO_TRANSPORT: &str = "the DEPOT control-plane transport (DEPOT-1: an IK-authenticated, \
Noise-secured channel to the coordinator, carrying the request in the service's adopted native \
protocol). No channel is opened, no request is built, and nothing is sent.";

/// The paths whose action is [`Action::Local`]. Enumerated so the mapping test can assert the
/// set has not silently grown — "it's local" must not become a way to escape the registry.
pub const LOCAL_ACTIONS: [&str; 2] = ["auth token", "auth whoami"];

/// **The table.** Every subcommand `pier` exposes.
pub const TABLE: &[Cmd] = &[
    // ---- box (§3, the compute elemental) --------------------------------------------------
    Cmd {
        path: &["box", "create"],
        about: "Provision a box from a declared shape and a DepotImage",
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::Provision,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["box", "list"],
        about: "Enumerate boxes in scope",
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::List,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["box", "show"],
        about: "Read one box's configuration and state",
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::Inspect,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["box", "start"],
        about: "Start a stopped box",
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::Start,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["box", "stop"],
        about: "Stop a running box",
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::Stop,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["box", "restart"],
        about: "Restart a box",
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::Restart,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["box", "snapshot"],
        about: "Capture a box snapshot (§4.1)",
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::Snapshot,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["box", "export"],
        about: "Obtain the DEPOT-4 portable export — the exit, and it is not optional",
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::Export,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["box", "destroy"],
        about: "Delete a box and release its resources",
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::Destroy,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["box", "console"],
        about: "Interactive access to a box — THE PRIVILEGE CLIFF (§5.2): subsumes nearly every \
                other ability, and a token carrying it must be delegated separately",
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::Console,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    // ---- volume ---------------------------------------------------------------------------
    Cmd {
        path: &["volume", "create"],
        about: "Provision a volume",
        action: Action::Depot {
            service: Some(Service::Volume),
            ability: Ability::Provision,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["volume", "list"],
        about: "Enumerate volumes in scope",
        action: Action::Depot {
            service: Some(Service::Volume),
            ability: Ability::List,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["volume", "attach"],
        about: "Attach a volume to a box",
        action: Action::Depot {
            service: Some(Service::Volume),
            ability: Ability::Attach,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["volume", "detach"],
        about: "Detach a volume from a box",
        action: Action::Depot {
            service: Some(Service::Volume),
            ability: Ability::Detach,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["volume", "resize"],
        about: "Grow a volume",
        action: Action::Depot {
            service: Some(Service::Volume),
            ability: Ability::Resize,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["volume", "snapshot"],
        about: "Capture a volume snapshot (§4.1)",
        action: Action::Depot {
            service: Some(Service::Volume),
            ability: Ability::Snapshot,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["volume", "export"],
        about: "Obtain the DEPOT-4 portable export",
        action: Action::Depot {
            service: Some(Service::Volume),
            ability: Ability::Export,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["volume", "destroy"],
        about: "Delete a volume — §5.3 requires the bytes be irrecoverable to the next tenant",
        action: Action::Depot {
            service: Some(Service::Volume),
            ability: Ability::Destroy,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    // ---- bucket ---------------------------------------------------------------------------
    Cmd {
        path: &["bucket", "create"],
        about: "Provision a bucket",
        action: Action::Depot {
            service: Some(Service::Bucket),
            ability: Ability::Provision,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["bucket", "list"],
        about: "Enumerate buckets in scope",
        action: Action::Depot {
            service: Some(Service::Bucket),
            ability: Ability::List,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["bucket", "put"],
        about: "Write an object",
        action: Action::Depot {
            service: Some(Service::Bucket),
            ability: Ability::Write,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["bucket", "get"],
        about: "Read an object",
        action: Action::Depot {
            service: Some(Service::Bucket),
            ability: Ability::Read,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["bucket", "rm"],
        about: "Delete an object",
        action: Action::Depot {
            service: Some(Service::Bucket),
            ability: Ability::Delete,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["bucket", "serve"],
        about: "Toggle public serving — the CDN mode (§3.7)",
        action: Action::Depot {
            service: Some(Service::Bucket),
            ability: Ability::Serve,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["bucket", "export"],
        about: "Obtain the DEPOT-4 portable export",
        action: Action::Depot {
            service: Some(Service::Bucket),
            ability: Ability::Export,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["bucket", "destroy"],
        about: "Delete a bucket and release its resources",
        action: Action::Depot {
            service: Some(Service::Bucket),
            ability: Ability::Destroy,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    // ---- fn (the edge-fn elemental) --------------------------------------------------------
    Cmd {
        path: &["fn", "deploy"],
        about: "Publish a new artefact version",
        action: Action::Depot {
            service: Some(Service::EdgeFn),
            ability: Ability::Deploy,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["fn", "invoke"],
        about: "Call the function",
        action: Action::Depot {
            service: Some(Service::EdgeFn),
            ability: Ability::Invoke,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["fn", "rollback"],
        about: "Point back at a previously deployed artefact",
        action: Action::Depot {
            service: Some(Service::EdgeFn),
            ability: Ability::Rollback,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["fn", "list"],
        about: "Enumerate functions in scope",
        action: Action::Depot {
            service: Some(Service::EdgeFn),
            ability: Ability::List,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["fn", "export"],
        about: "Obtain the DEPOT-4 portable export",
        action: Action::Depot {
            service: Some(Service::EdgeFn),
            ability: Ability::Export,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["fn", "destroy"],
        about: "Delete a function and release its resources",
        action: Action::Depot {
            service: Some(Service::EdgeFn),
            ability: Ability::Destroy,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    // ---- cross-cutting ---------------------------------------------------------------------
    Cmd {
        path: &["logs"],
        about: "Read logs and metrics for an instance (§4.3 / DEPOT-14: OTLP)",
        action: Action::Depot {
            service: None,
            ability: Ability::Observe,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["status"],
        about: "Enumerate every DEPOT instance at this coordinator (scope depot:*)",
        action: Action::Depot {
            service: None,
            ability: Ability::List,
        },
        status: Status::Scaffold,
        missing: NO_TRANSPORT,
    },
    Cmd {
        path: &["certs", "add"],
        about: "Add a TLS certificate for a domain, with the BOX holding the private key \
                (REACH-2a). Prints the tier, what it costs you, and the DNS records to publish",
        // Certificate issuance is REACH, not a DEPOT elemental (§3.4: "DEPOT adds no networking
        // primitives"). The DEPOT-side act is mutating the box's declared configuration to serve
        // a new public name, which is `reconfigure` on `depot:box/<id>` — not a coined verb.
        action: Action::Depot {
            service: Some(Service::Box),
            ability: Ability::Reconfigure,
        },
        status: Status::LocalOnly,
        missing: "the ACME run itself. This command plans; it does not issue. See `certs add \
--help` for the four things that are not built.",
    },
    // ---- auth: local by construction --------------------------------------------------------
    Cmd {
        path: &["auth", "token"],
        about: "Mint a CapabilityToken (§18.7.3) scoped to a DEPOT resource and ability",
        action: Action::Local,
        status: Status::Scaffold,
        missing: "capability minting. Minting requires an issuer IdentityKey this CLI has no \
keystore for, and a parent token to attenuate from. `pier` will not fall back to a bearer API \
key, so there is nothing to print.",
    },
    Cmd {
        path: &["auth", "whoami"],
        about: "Decode and verify the CapabilityToken in use, and print exactly what it grants",
        action: Action::Local,
        status: Status::LocalOnly,
        missing: "",
    },
];

/// Look a row up by its joined path.
pub fn lookup(key: &str) -> Option<&'static Cmd> {
    TABLE.iter().find(|c| c.key() == key)
}

/// The distinct top-level groups, in table order — `box`, `volume`, `bucket`, `fn`, `certs`,
/// `auth` — plus the single-word commands `logs` and `status`.
pub fn groups() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for c in TABLE {
        let head = c.path[0];
        if !out.contains(&head) {
            out.push(head);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_path_is_unique() {
        let mut seen: Vec<String> = Vec::new();
        for c in TABLE {
            let k = c.key();
            assert!(!seen.contains(&k), "duplicate command path {k:?}");
            seen.push(k);
        }
        assert_eq!(seen.len(), TABLE.len());
    }

    #[test]
    fn paths_are_one_or_two_words() {
        for c in TABLE {
            assert!(
                c.path.len() == 1 || c.path.len() == 2,
                "{:?} — the clap builder only nests one level",
                c.path
            );
            for seg in c.path {
                assert!(!seg.is_empty() && !seg.contains(' '), "bad segment {seg:?}");
            }
        }
    }

    #[test]
    fn help_line_shows_the_wire_verb_and_the_status() {
        let destroy = lookup("box destroy").unwrap();
        let line = destroy.help_line();
        assert!(line.contains("[ability: destroy]"), "{line}");
        assert!(line.contains("SCAFFOLD"), "{line}");
        // A local row has no ability to show, and must not invent one.
        let whoami = lookup("auth whoami").unwrap();
        assert!(
            !whoami.help_line().contains("ability:"),
            "{}",
            whoami.help_line()
        );
    }
}
