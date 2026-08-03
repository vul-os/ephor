//! **`pier`** — a flyctl-shaped CLI over the KOTVA **DEPOT** control plane
//! ([`profiles/cloud.md`](https://github.com/vul-os/kotva/blob/main/profiles/cloud.md) §5).
//!
//! # Status, first
//!
//! There is **no transport**. Not one subcommand performs a real operation against a coordinator:
//! DEPOT-1's IK-authenticated, Noise-secured channel is not built, so nothing is ever sent. Every
//! command says so when it runs and in `--help`, and the ones that cannot finish exit non-zero.
//! See [`table::Status`].
//!
//! What is real:
//!
//! * **The vocabulary.** Every action in [`table::TABLE`] carries a [`kotva_depot::Ability`]
//!   *value* from the closed §5.2 registry — not a string — and every scope is a
//!   [`kotva_depot::ResourceRef`] from the §5.1 grammar. `tests/ability_mapping.rs` walks the
//!   built clap tree back through the table in both directions, with counts, so this cannot rot.
//! * **The authorisation path**, and its refusals: [`auth`]. A §18.7.3 `CapabilityToken` bound to
//!   one coordinator by the §5.1 `depot:coordinator` caveat. No bearer keys, no anonymous
//!   fallback, no "try it and see".
//! * **The `certs add` planner**: [`certs`]. The §3.4 tier where the **box** holds the private
//!   key, the exact DNS records it needs, the honest `declared`-vs-`structural` call, and a blunt
//!   list of the five things about ACME that are not built.
//!
//! # Why the verb vocabulary is a type and not a string
//!
//! §5.2's registry is closed *to coinage*: a coordinator receiving an ability outside it MUST
//! refuse and MUST NOT map it onto a similar-sounding one — "an operator MUST NOT coin
//! `terminate` for `destroy`". That rule is what lets one open-source client drive any conformant
//! gateway, and a client is the party most likely to break it, because a CLI's user-facing nouns
//! (`create`, `show`, `rm`) are chosen for ergonomics. So the CLI keeps the flyctl-shaped noun and
//! the table maps it to an [`Ability`](kotva_depot::Ability) variant; `--help` prints both, so the
//! translation is documented rather than folklore.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod auth;
pub mod certs;
pub mod cli;
pub mod run;
pub mod table;

pub use run::{run, Invocation};
pub use table::{Action, Cmd, Status, TABLE};
