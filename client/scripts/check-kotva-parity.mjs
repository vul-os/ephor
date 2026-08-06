#!/usr/bin/env node
/**
 * check-kotva-parity.mjs — the substrate modules in src/ are KOTVA's, and this
 * proves none of them has been edited here.
 *
 * WHY THIS EXISTS. `src/chunkProof.js` used to open by declaring itself "the
 * BROWSER half of the DMTAP-PUB § 5.3 chunk-tree range proof" whose "EXACT
 * PARITY WITH THE GO IMPLEMENTATION is the whole point". A wire format with two
 * implementations, in two repos, with no shared owner, drifts — silently, and in
 * the direction that breaks interop long after the change that caused it. The
 * modules now belong to the `kotva-client` package (kotva/bindings/js); the
 * copies here are pinned to a tag and this gate fails the moment one of them
 * stops matching.
 *
 * SELF-CONTAINED BY DESIGN. It compares against sha256 hashes recorded in
 * kotva-client.pin.json, NOT against a sibling ../kotva checkout. That is
 * deliberate: this repo already documents the cost of the sibling-checkout
 * pattern in crates/pier-cli/Cargo.toml, where a path dep on kotva-depot makes
 * `cargo build` at the root require the sibling to exist. Hashes need no
 * checkout and no network, so this runs identically in CI, offline, and on a
 * fresh clone — and it CANNOT degrade into the failure mode where a gate skips
 * itself because its subject is missing and reports success anyway.
 *
 * WHAT IT DOES NOT DO. It detects drift in THIS repo's copies. It does not
 * notice that kotva has moved on — that is what pinning to a tag means. Re-pin
 * deliberately with `npm run kotva:pin -- <path-to-kotva-checkout>`.
 */

import { createHash } from 'node:crypto'
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { join, dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

/**
 * The shape of kotva-client.pin.json, typed narrowly to just the fields this
 * script reads/writes — a `@typedef` so `JSON.parse`'s `any` doesn't disable
 * checking on everything downstream (e.g. `pin.files[name]` resolving to
 * `any` and silently swallowing a typo'd field name).
 * @typedef {Object} KotvaPin
 * @property {string} package
 * @property {string} version
 * @property {string} repository
 * @property {string} tag
 * @property {string} commit
 * @property {string} source_dir
 * @property {string} target_dir
 * @property {Record<string, string>} files
 */

const CLIENT_DIR = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const PIN_PATH = join(CLIENT_DIR, 'kotva-client.pin.json')

/**
 * @param {Buffer} buf
 * @returns {string}
 */
const sha256 = (buf) => createHash('sha256').update(buf).digest('hex')

/** @returns {KotvaPin} */
function loadPin() {
  if (!existsSync(PIN_PATH)) {
    console.error(`FAIL kotva parity: ${PIN_PATH} is missing.`)
    console.error('      The pin IS the gate; without it nothing is being checked.')
    process.exit(1)
  }
  /** @type {KotvaPin} */
  const pin = JSON.parse(readFileSync(PIN_PATH, 'utf8'))
  const names = Object.keys(pin.files ?? {})
  // A pin that lists nothing would pass every check while checking nothing.
  // Refuse to be that gate.
  if (names.length === 0) {
    console.error('FAIL kotva parity: the pin lists zero files, so it verifies nothing.')
    process.exit(1)
  }
  return pin
}

/**
 * `npm run kotva:pin -- <kotva-checkout>` — re-sync from kotva and rewrite the pin.
 * @param {string} kotvaDir
 * @returns {void}
 */
function repin(kotvaDir) {
  const pin = loadPin()
  const srcDir = join(resolve(kotvaDir), pin.source_dir)
  if (!existsSync(srcDir)) {
    console.error(`FAIL: ${srcDir} does not exist — is that a kotva checkout?`)
    process.exit(1)
  }
  /** @type {Record<string, string>} */
  const next = {}
  for (const name of Object.keys(pin.files)) {
    const from = join(srcDir, name)
    if (!existsSync(from)) {
      console.error(`FAIL: ${name} is pinned but absent from ${srcDir}.`)
      console.error('      Removing a module upstream is a deliberate act; edit the pin by hand.')
      process.exit(1)
    }
    const buf = readFileSync(from)
    writeFileSync(join(CLIENT_DIR, pin.target_dir, name), buf)
    next[name] = sha256(buf)
    const changed = next[name] !== pin.files[name]
    console.log(`  ${changed ? 'updated' : 'unchanged'}  ${name}`)
  }
  pin.files = next
  writeFileSync(PIN_PATH, JSON.stringify(pin, null, 2) + '\n')
  console.log(`\nRe-synced from ${srcDir}.`)
  console.log('Now set `tag`, `commit` and `version` in kotva-client.pin.json to the tag you')
  console.log('synced from — the hashes moved, and a stale tag makes the pin a lie.')
}

/** @returns {void} */
function check() {
  const pin = loadPin()
  /** @type {string[]} */
  const bad = []
  let n = 0
  for (const [name, want] of Object.entries(pin.files)) {
    const path = join(CLIENT_DIR, pin.target_dir, name)
    if (!existsSync(path)) {
      bad.push(`${name}: pinned but MISSING from ${pin.target_dir}/`)
      continue
    }
    const got = sha256(readFileSync(path))
    if (got !== want) bad.push(`${name}:\n      pinned ${want}\n      found  ${got}`)
    n++
  }

  if (bad.length) {
    console.error(`\nFAIL kotva parity — ${bad.length} of ${Object.keys(pin.files).length} substrate module(s) drifted from ${pin.tag}:\n`)
    for (const b of bad) console.error(`  • ${b}`)
    console.error(`\nThese files belong to ${pin.package}@${pin.version} (${pin.repository}).`)
    console.error('Do not fix them here. Fix them in kotva, cut a new bindings/js tag, then:')
    console.error('    npm run kotva:pin -- ../../kotva\n')
    process.exit(1)
  }

  console.log(`kotva parity OK — ${n} substrate module(s) byte-identical to ${pin.package}@${pin.version} (${pin.tag}).`)
}

const [, , cmd, arg] = process.argv
if (cmd === '--repin') {
  if (!arg) {
    console.error('usage: npm run kotva:pin -- <path-to-kotva-checkout>')
    process.exit(1)
  }
  repin(arg)
} else {
  check()
}
