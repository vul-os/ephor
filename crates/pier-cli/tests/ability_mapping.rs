//! **The enforcement test.** Every verb this CLI exposes must resolve to a
//! `kotva_depot::Ability` from the CLOSED §5.2 registry — there is no CLI-local verb vocabulary.
//!
//! # What it walks, and why that matters
//!
//! It does **not** iterate the table and check the table. It builds the real `clap::Command` tree
//! (the thing a user actually types into), walks it to its leaves, and resolves each leaf *back*
//! through the table — then does the reverse walk as well. Both directions, because the recurring
//! failure in this suite is a check that only ever goes source→dest: a table row that stopped
//! being reachable from the CLI would pass a one-directional test forever.
//!
//! # Why the counts
//!
//! A walker with a bug that returns nothing makes every `for` loop below vacuously true, and the
//! test prints PASS having examined zero commands. So: the leaf count is asserted against a floor,
//! the ability count is asserted against a floor, the local-action set is asserted **exactly**,
//! and the walker itself is run against an empty `Command` to prove it can return zero — a
//! false-positive control for the instrument, not for the subject.

use std::collections::BTreeSet;

use kotva_depot::{Ability, Service};
use pier_cli::table::{Action, LOCAL_ACTIONS, TABLE};

/// Walk a clap tree to its leaves, returning space-joined paths (`"box destroy"`).
fn leaves(cmd: &clap::Command) -> BTreeSet<String> {
    fn walk(c: &clap::Command, prefix: &str, out: &mut BTreeSet<String>) {
        let subs: Vec<&clap::Command> = c
            .get_subcommands()
            .filter(|s| s.get_name() != "help")
            .collect();
        if subs.is_empty() {
            if !prefix.is_empty() {
                out.insert(prefix.to_string());
            }
            return;
        }
        for s in subs {
            let path = if prefix.is_empty() {
                s.get_name().to_string()
            } else {
                format!("{prefix} {}", s.get_name())
            };
            walk(s, &path, out);
        }
    }
    let mut c = cmd.clone();
    c.build();
    let mut out = BTreeSet::new();
    walk(&c, "", &mut out);
    out
}

/// FALSE-POSITIVE CONTROL for the instrument itself. If `leaves` returned a constant, or silently
/// swallowed its recursion, every assertion below would be vacuous. Prove it can return zero, and
/// prove it finds a nested leaf it would have to recurse to reach.
#[test]
fn the_walker_is_not_a_constant() {
    assert!(leaves(&clap::Command::new("empty")).is_empty());

    let toy = clap::Command::new("toy")
        .subcommand(clap::Command::new("flat"))
        .subcommand(clap::Command::new("group").subcommand(clap::Command::new("deep")));
    let got = leaves(&toy);
    assert_eq!(
        got,
        ["flat", "group deep"]
            .iter()
            .map(|s| s.to_string())
            .collect::<BTreeSet<_>>(),
        "the walker must reach nested leaves and must not emit the group itself"
    );
}

#[test]
fn every_cli_leaf_resolves_to_a_table_row_and_every_row_is_reachable() {
    let cli_leaves = leaves(&pier_cli::cli::build());
    let table_keys: BTreeSet<String> = TABLE.iter().map(|c| c.key()).collect();

    // Direction 1: nothing typeable is missing a row.
    let orphan_commands: Vec<&String> = cli_leaves.difference(&table_keys).collect();
    assert!(
        orphan_commands.is_empty(),
        "these CLI commands have no table row, so they map to no ability: {orphan_commands:?}"
    );
    // Direction 2: nothing in the table is unreachable. A row that stops being wired up is a row
    // whose ability mapping is no longer being enforced by anything.
    let unreachable_rows: Vec<&String> = table_keys.difference(&cli_leaves).collect();
    assert!(
        unreachable_rows.is_empty(),
        "these table rows are not reachable from the CLI: {unreachable_rows:?}"
    );

    assert_eq!(cli_leaves.len(), table_keys.len());
    assert_eq!(table_keys.len(), TABLE.len(), "duplicate keys in the table");
    // COVERAGE FLOOR. Raise it when commands are added; lowering it to make a red test pass is
    // the failure this line exists to make visible.
    assert!(
        cli_leaves.len() >= 37,
        "only {} commands walked — the tree shrank or the walk stopped early",
        cli_leaves.len()
    );
}

#[test]
fn every_action_resolves_to_an_ability_in_the_closed_registry() {
    let mut depot_rows = 0usize;
    let mut local_rows: Vec<String> = Vec::new();

    for cmd in TABLE {
        match cmd.action {
            Action::Depot { service, ability } => {
                // 1. The verb round-trips through the CLOSED registry. `Ability::from_str` is the
                //    coordinator-side parser: if it returns None the coordinator would refuse.
                assert_eq!(
                    Ability::from_str(ability.as_str()),
                    Some(ability),
                    "`{}` carries a verb the §5.2 registry does not parse",
                    cmd.key()
                );
                // 2. The verb is meaningful for the elemental it is scoped to (§5.2 per-elemental
                //    table) — `attach` on an edge-fn is as wrong as a coined verb.
                if let Some(s) = service {
                    assert!(
                        ability.is_valid_for(s),
                        "`{}` maps {} onto {:?}, which §5.2 does not permit",
                        cmd.key(),
                        ability.as_str(),
                        s
                    );
                }
                depot_rows += 1;
            }
            Action::Local => local_rows.push(cmd.key()),
        }
    }

    // COVERAGE. A table that walked nothing cannot pass.
    assert!(depot_rows >= 35, "only {depot_rows} DEPOT rows checked");
    // The local escape hatch is fenced EXACTLY, not by a floor: "it's local" must not become the
    // way a CLI-local verb gets in.
    let expected: BTreeSet<String> = LOCAL_ACTIONS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        local_rows.into_iter().collect::<BTreeSet<_>>(),
        expected,
        "the set of commands that bypass the ability registry changed"
    );
    assert_eq!(depot_rows + LOCAL_ACTIONS.len(), TABLE.len());
}

#[test]
fn no_coined_verb_ever_reaches_the_wire() {
    // The §5.2 example is explicit: "An operator MUST NOT coin `terminate` for `destroy`". These
    // are the CLI's own nouns and the coinages a catalogue-minded implementer reaches for. None
    // of them may appear as a wire verb, however friendly the subcommand that produced it.
    const FORBIDDEN: [&str; 14] = [
        "terminate",
        "reboot",
        "create",
        "remove",
        "exec",
        "ssh",
        "put",
        "get",
        "rm",
        "show",
        "status",
        "logs",
        "add",
        "destroy-box",
    ];
    let mut wire_verbs: BTreeSet<&str> = BTreeSet::new();
    for cmd in TABLE {
        if let Some(a) = cmd.ability() {
            wire_verbs.insert(a.as_str());
        }
    }
    assert!(!wire_verbs.is_empty());
    for bad in FORBIDDEN {
        assert!(
            !wire_verbs.contains(bad),
            "{bad:?} reached the wire-verb set"
        );
        // ...and the registry itself must not parse it either, which is what makes the CLI's
        // restraint enforceable rather than polite.
        assert_eq!(
            Ability::from_str(bad),
            None,
            "{bad:?} unexpectedly parses as an ability"
        );
    }

    // The prompt's own example, pinned: the friendly noun is `destroy` and so is the wire verb.
    let d = pier_cli::table::lookup("box destroy").unwrap();
    assert_eq!(d.ability(), Some(Ability::Destroy));
    assert_eq!(d.ability().unwrap().as_str(), "destroy");
    // And the ones where the friendly noun deliberately differs still land on the real verb.
    for (key, want) in [
        ("box create", Ability::Provision),
        ("box show", Ability::Inspect),
        ("bucket put", Ability::Write),
        ("bucket get", Ability::Read),
        ("bucket rm", Ability::Delete),
        ("logs", Ability::Observe),
        ("certs add", Ability::Reconfigure),
    ] {
        assert_eq!(
            pier_cli::table::lookup(key).unwrap().ability(),
            Some(want),
            "`{key}` drifted"
        );
    }
}

#[test]
fn the_help_text_states_the_wire_verb_for_every_depot_command() {
    // The mapping is only useful to a user if they can see it without reading this source.
    let mut checked = 0usize;
    for cmd in TABLE {
        if let Some(a) = cmd.ability() {
            let line = cmd.help_line();
            assert!(
                line.contains(&format!("[ability: {}]", a.as_str())),
                "`{}` help does not name its wire verb: {line}",
                cmd.key()
            );
            checked += 1;
        }
    }
    assert!(checked >= 35, "only {checked} help lines checked");

    // ...and it survives clap's rendering, which is what a user actually sees.
    let mut root = pier_cli::cli::build();
    let boxes = root.find_subcommand_mut("box").expect("box group");
    let rendered = boxes.render_long_help().to_string();
    assert!(rendered.contains("[ability: destroy]"), "{rendered}");
    assert!(rendered.contains("[ability: provision]"), "{rendered}");
    assert!(rendered.contains("[SCAFFOLD: sends nothing]"), "{rendered}");
}

#[test]
fn the_console_privilege_cliff_is_marked_and_singular() {
    let cliff: Vec<String> = TABLE
        .iter()
        .filter(|c| {
            c.ability()
                .is_some_and(|a| a.requires_separate_delegation())
        })
        .map(|c| c.key())
        .collect();
    assert_eq!(cliff, vec!["box console".to_string()]);
    assert!(pier_cli::table::lookup("box console")
        .unwrap()
        .about
        .contains("PRIVILEGE CLIFF"));
}

#[test]
fn the_cli_offers_export_wherever_it_offers_destroy() {
    // A client-side echo of §5.2's operator obligation: "An operator MUST NOT offer `destroy`
    // while withholding `export`." A CLI that can delete an instance but has no command to
    // extract it has made the exit weaker than the loss (DEPOT-4), whatever the coordinator
    // offers. Checked per elemental, since that is the granularity a user experiences.
    let mut checked = 0usize;
    for s in Service::ALL {
        let has = |a: Ability| {
            TABLE
                .iter()
                .any(|c| c.service() == Some(s) && c.ability() == Some(a))
        };
        if has(Ability::Destroy) {
            assert!(
                has(Ability::Export),
                "{s:?}: `destroy` is offered but `export` is not"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 4, "expected all four elementals to offer destroy");
}

#[test]
fn every_elemental_is_reachable_from_the_cli() {
    // Guards the inverse of the coverage floor: the count could be met while a whole elemental
    // went missing.
    for s in Service::ALL {
        let n = TABLE.iter().filter(|c| c.service() == Some(s)).count();
        assert!(n >= 6, "{s:?} has only {n} commands");
    }
}
