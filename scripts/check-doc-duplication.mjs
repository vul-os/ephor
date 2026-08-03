#!/usr/bin/env node
//
// check-doc-duplication.mjs — docs/ and site/docs/ hold the SAME documents twice.
//
// There is no generator and no sync step: site/docs/architecture.md is a hand-kept
// copy of docs/ARCHITECTURE.md, and so on for a dozen pairs. That arrangement does
// not fail loudly, it fails QUIETLY — an edit lands in one copy, the other keeps
// serving the old text, and nothing anywhere says the two disagree. When this check
// was first written 2 of 12 pairs had already drifted, and one of them meant docs/
// carried a duplicated word ("the full roster / roster,") that the site copy had
// silently fixed months earlier.
//
// So: every pair must be byte-identical UNLESS the difference is DECLARED below
// with a reason. A declared difference is a decision; an undeclared one is a bug.
//
// This deliberately does not "fix" divergence by copying one side over the other.
// Which side is right is a judgement call the checker cannot make — see the
// screenshots.md entry, where the site copy de-brands wording the client code still
// uses. The job here is to make that visible and force it to be answered, not to
// guess.

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SITE_DOCS = join(REPO, "site", "docs");
const DOCS = join(REPO, "docs");

// Pairs whose two copies are ALLOWED to differ, each with the reason. Keep this
// list short and argued; it is the part of the file a reviewer should read.
const DECLARED_DIVERGENCE = {
  "getting-started.md":
    "The docs/ copy links to CLOUD-SETUP.md, which is not published to site/docs/. " +
    "Carrying the paragraph onto the site would ship a dead link.",
  // screenshots.md was the second entry here and is deliberately GONE: the two
  // copies documented different products, and the docs/ side has now been
  // deleted and replaced by the site copy verbatim. See the CHANGELOG entry —
  // demo/index.html and the two foreign-brand PNGs under docs/screenshots/ were
  // removed outright. The pair is byte-identical again, so this checker's own
  // stale-exception rule (below) would fail the run if the entry were left here.
};

// A pair count floor, so an empty or broken glob cannot read as "all clean".
const MIN_PAIRS = 12;

const red = (s) => `\x1b[31m${s}\x1b[0m`;
const grn = (s) => `\x1b[32m${s}\x1b[0m`;

if (!existsSync(SITE_DOCS)) {
  console.error(red("FAIL") + ` no such directory: ${SITE_DOCS}`);
  process.exit(1);
}

// site/docs/foo-bar.md  ->  docs/FOO-BAR.md
//
// Only the STEM is uppercased. A bare `siteName.toUpperCase()` yields
// "ARCHITECTURE.MD", which existsSync() happily resolves on macOS's
// case-insensitive filesystem and which matches nothing on CI's Linux — a check
// that passes on the author's laptop and means something different on the runner.
const docsNameFor = (siteName) => siteName.replace(/\.md$/, "").toUpperCase() + ".md";

// Resolve against the real directory listing rather than existsSync, so the
// pairing is case-EXACT on every platform instead of only on case-sensitive ones.
const docsEntries = new Set(readdirSync(DOCS));

const siteFiles = readdirSync(SITE_DOCS)
  .filter((f) => f.endsWith(".md"))
  .sort();

const problems = [];
let identical = 0;
let declared = 0;
const checked = [];

for (const name of siteFiles) {
  const sitePath = join(SITE_DOCS, name);
  const docsPath = join(DOCS, docsNameFor(name));

  if (!docsEntries.has(docsNameFor(name))) {
    // An orphan is not automatically wrong, but it must be noticed: it means the
    // site publishes a document the repo's own docs/ tree does not have.
    problems.push(
      `${name}: no docs/ counterpart (looked for docs/${docsNameFor(name)}). ` +
        `Either it is site-only (say so here) or the pairing rule is wrong.`,
    );
    continue;
  }

  checked.push(name);
  const a = readFileSync(docsPath);
  const b = readFileSync(sitePath);

  if (a.equals(b)) {
    identical += 1;
    if (name in DECLARED_DIVERGENCE) {
      // A stale exception is its own defect: it tells future readers a difference
      // exists and is intended, when in fact the copies now agree.
      problems.push(
        `${name}: listed in DECLARED_DIVERGENCE but the two copies are now IDENTICAL. ` +
          `Remove the entry — a declared difference that does not exist misleads.`,
      );
    }
    continue;
  }

  if (name in DECLARED_DIVERGENCE) {
    declared += 1;
    continue;
  }

  problems.push(
    `${name}: docs/${docsNameFor(name)} and site/docs/${name} DIFFER, undeclared.\n` +
      `      Diff them, decide which side is right, fix that side — or, if the\n` +
      `      difference is deliberate, add it to DECLARED_DIVERGENCE with a reason.`,
  );
}

// Coverage. Counting is not checking: assert the scan actually found the corpus.
if (checked.length < MIN_PAIRS) {
  console.error(
    red("FAIL") +
      ` only ${checked.length} doc pair(s) were compared, expected at least ${MIN_PAIRS}.` +
      `\n      An empty or truncated glob must not read as "everything matches".`,
  );
  process.exit(1);
}

if (problems.length > 0) {
  console.error(red("FAIL") + ` ${problems.length} doc-duplication problem(s):\n`);
  for (const p of problems) console.error(`  - ${p}`);
  console.error(
    `\n  ${checked.length} pairs compared: ${identical} identical, ${declared} declared-divergent.`,
  );
  process.exit(1);
}

console.log(
  grn("  ok") +
    ` ${checked.length} doc pairs compared: ${identical} byte-identical, ` +
    `${declared} divergent-by-declaration (${Object.keys(DECLARED_DIVERGENCE).join(", ")}).`,
);
console.log(
  "       docs/ and site/docs/ are duplicated by hand — there is no generator. " +
    "This check is the only thing keeping them honest.",
);
