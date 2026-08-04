// relayBox.d.ts — hand-written type declarations for the pinned kotva-client
// substrate module relayBox.js (see kotva-client.pin.json).
// See errors.d.ts for why this file exists and the rule it follows.

export interface BoxKeyPair {
  privateKey: Uint8Array
  publicKey: Uint8Array
  publicKeyB64: string
}

export declare function bytesToB64(bytes: Uint8Array): string
export declare function b64ToBytes(b64: string): Uint8Array

/** Generate a fresh X25519 box keypair. */
export declare function generateBoxKeyPair(): BoxKeyPair

export interface SealRelayBlobArgs {
  /** UTF-8/binary payload bytes */
  plaintext: Uint8Array
  /** sender X25519 private key */
  senderBoxPriv: Uint8Array
  /** recipient X25519 public key (base64) */
  recipientBoxPubB64: string
  from: string
  to: string
  session: string
}

/**
 * Encrypt `plaintextBytes` for `recipientBoxPubB64`.
 * @returns blob_b64  (version || nonce || ciphertext+tag), base64
 */
export declare function sealRelayBlob(args: SealRelayBlobArgs): string

export interface OpenRelayBlobArgs {
  /** the blob_b64 from the relay */
  blobB64: string
  /** this peer's X25519 private key */
  recipientBoxPriv: Uint8Array
  /** sender X25519 public key (base64, the blob `epk`) */
  senderBoxPubB64: string
  from: string
  to: string
  session: string
}

/**
 * Decrypt a blob produced by sealRelayBlob.
 * @returns plaintext bytes
 * @throws on version mismatch, truncation, wrong key or tamper (AEAD failure)
 */
export declare function openRelayBlob(args: OpenRelayBlobArgs): Uint8Array

/** Read the blob version byte (1 or 2) without decrypting. */
export declare function relayBlobVersion(blobB64: string): number

export interface SealRelayBlobV2Args {
  plaintext: Uint8Array
  /** 32-byte X3DH SK (from x3dhInitiate) */
  key: Uint8Array
  /** sender X25519 ephemeral public key */
  ephemeralPub: Uint8Array
  /** recipient signed-prekey id used */
  signedPreKeyId: string
  /** recipient one-time-prekey id, or null */
  oneTimePreKeyId: string | null
  from: string
  to: string
  session: string
}

/**
 * Seal `plaintext` under a v2 (X3DH) content key.
 * @returns blob_b64
 */
export declare function sealRelayBlobV2(args: SealRelayBlobV2Args): string

export interface ParsedRelayBlobV2 {
  version: 2
  ephemeralPub: Uint8Array
  signedPreKeyId: string
  oneTimePreKeyId: string | null
  headerBytes: Uint8Array
  nonce: Uint8Array
  sealed: Uint8Array
}

/**
 * Parse a v2 blob's clear header WITHOUT decrypting, so the recipient can pick
 * the right prekeys and run X3DHRespond before opening.
 */
export declare function parseRelayBlobV2(blobB64: string): ParsedRelayBlobV2

export interface OpenRelayBlobV2Args {
  parsed: ParsedRelayBlobV2
  /** 32-byte X3DH SK (from x3dhRespond) */
  key: Uint8Array
  from: string
  to: string
  session: string
}

/**
 * Open a v2 blob given the X3DH SK derived from its parsed header.
 * @returns plaintext
 * @throws on tamper / wrong key (AEAD failure)
 */
export declare function openRelayBlobV2(args: OpenRelayBlobV2Args): Uint8Array
