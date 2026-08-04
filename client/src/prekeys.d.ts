// prekeys.d.ts — hand-written type declarations for the pinned kotva-client
// substrate module prekeys.js (see kotva-client.pin.json).
// See errors.d.ts for why this file exists and the rule it follows.
//
// prekeys.js is the browser/JS port of the Go X3DH reference (prekeys.go);
// see its module docstring for the full DH1..DH4/HKDF derivation. This file
// types the public API only — it does not restate the algorithm.

/** Signs raw bytes with the ECDSA identity key, returning a base64 signature. */
export type SignRawFn = (msgBytes: Uint8Array) => Promise<string>

/** Verifies a base64 ECDSA signature over raw bytes. */
export type VerifyRawFn = (msgBytes: Uint8Array, sigB64: string) => Promise<boolean>

/**
 * Derive the 32-byte content key from the DH concatenation and the two
 * identity ids. Matches `prekeys.go:x3dhKDF` byte-for-byte.
 */
export declare function x3dhKDF(dhConcat: Uint8Array, idA: string, idB: string): Uint8Array

/** Arguments to {@link x3dhInitiate}. */
export interface X3dhInitiateArgs {
  /** sender long-term (box) X25519 private — IK_send */
  identityPriv: Uint8Array
  /** recipient long-term (box) X25519 public — IK_recv */
  recipientIdentityPub: Uint8Array
  /** recipient signed prekey public — SPK_recv */
  signedPreKeyPub: Uint8Array
  /** recipient one-time prekey public — OPK_recv */
  oneTimePreKeyPub?: Uint8Array | null
  /** sender identity id (salt input) */
  senderId: string
  /** recipient identity id (salt input) */
  recipientId: string
}

/** Result of {@link x3dhInitiate}. */
export interface X3dhInitiateResult {
  ephemeralPub: Uint8Array
  sk: Uint8Array
}

/**
 * Derive a forward-secret content key SK for sending to a recipient bundle.
 * Mirrors `prekeys.go:X3DHInitiate`. Generates a fresh ephemeral key.
 */
export declare function x3dhInitiate(args: X3dhInitiateArgs): X3dhInitiateResult

/** Arguments to {@link x3dhRespond}. */
export interface X3dhRespondArgs {
  /** recipient long-term (box) X25519 private — IK_recv */
  identityPriv: Uint8Array
  /** sender long-term (box) X25519 public — IK_send */
  senderIdentityPub: Uint8Array
  /** recipient signed prekey private — SPK_recv */
  signedPreKeyPriv: Uint8Array | null
  /** recipient one-time prekey private — OPK_recv */
  oneTimePreKeyPriv?: Uint8Array | null
  /** sender ephemeral public — EK */
  ephemeralPub: Uint8Array
  senderId: string
  recipientId: string
}

/**
 * Re-derive the same SK on the recipient side. Mirrors `prekeys.go:X3DHRespond`.
 * The one-time prekey (when used) MUST be deleted by the caller AFTER a
 * successful AEAD open — this function returns the SK only.
 */
export declare function x3dhRespond(args: X3dhRespondArgs): Uint8Array

/** A signed prekey's PRIVATE record, as held by {@link PreKeyStore}. */
export interface SignedPreKey {
  id: string
  priv: Uint8Array
  pub: Uint8Array
  pubB64: string
  sigB64: string
}

/** The publishable {id, pub, sig} form of a signed prekey. */
export interface SignedPreKeyPublic {
  id: string
  pub: string
  sig: string
}

/**
 * Generate a signed prekey: a fresh X25519 keypair whose PUBLIC key is signed
 * by the session ECDSA identity.
 */
export declare function generateSignedPreKey(signRawFn: SignRawFn): Promise<SignedPreKey>

/**
 * Verify a peer's signed prekey signature using the peer's stored ECDSA
 * public key. Fails closed (returns false on any error).
 */
export declare function verifySignedPreKey(
  verifyRawFn: VerifyRawFn,
  signedPreKey: { pub: string, sig: string },
): Promise<boolean>

/** The publishable prekey bundle produced by {@link PreKeyStore.publicBundle}. */
export interface PublicPreKeyBundle {
  identity_vula_id: string
  signed_prekey: SignedPreKeyPublic
  one_time_prekeys: Array<{ id: string, pub: string }>
}

/**
 * Owns this peer's PRIVATE prekey material (signed prekey + one-time prekey
 * pool) and produces the publishable bundle. In-memory / per-session.
 * Consumed one-time prekeys are deleted (forward secrecy).
 */
export declare class PreKeyStore {
  constructor(opts: { signedPreKey: SignedPreKey, oneTimeCount?: number })

  /** Build a store, generating + signing a fresh signed prekey. */
  static create(signRawFn: SignRawFn, oneTimeCount?: number): Promise<PreKeyStore>

  /** Top the one-time prekey pool back up to `target`. */
  replenish(target: number): void

  /** Number of one-time prekeys remaining. */
  oneTimePreKeyCount(): number

  /** The signed prekey id. */
  readonly signedPreKeyId: string

  /** The publishable bundle (signed prekey + remaining one-time prekey PUBLICS). */
  publicBundle(identityVulaId: string): PublicPreKeyBundle

  /** Private scalar of the signed prekey if `id` matches, else null. */
  signedPreKeyPriv(id: string): Uint8Array | null

  /** Private scalar of a one-time prekey by id, else null (no deletion). */
  oneTimePreKeyPriv(id: string | null | undefined): Uint8Array | null

  /** Delete a one-time prekey (forward secrecy). Returns true if it existed. */
  consumeOneTimePreKey(id: string | null | undefined): boolean
}
