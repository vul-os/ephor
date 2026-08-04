// rendezvous.d.ts — hand-written type declarations for the pinned kotva-client
// substrate module rendezvous.js (see kotva-client.pin.json).
// See errors.d.ts for why this file exists and the rule it follows.
//
// rendezvous.js is the reference JS client for the OPEN rendezvous role served
// by any vulos-relayd under its /rendezvous prefix (announce / resolve /
// signal / mailbox + ICE, Ed25519-signed). See its module docstring for the
// full protocol.

/** Encode bytes to unpadded base64url. */
export declare function b64urlEncode(bytes: Uint8Array | ArrayBufferLike): string

/** Decode unpadded base64url to bytes. */
export declare function b64urlDecode(str: string): Uint8Array

/**
 * Build the canonical signing message: the domain tag then each field, each
 * as a 4-byte big-endian length + UTF-8 bytes. Byte-for-byte identical to the
 * Go node's canonicalMessage().
 */
export declare function canonicalMessage(domain: string, fields: Array<string | number>): Uint8Array

/**
 * An Ed25519 identity used to sign rendezvous writes. Wrap an existing
 * 32-byte secret key, or call RendezvousIdentity.generate() for a fresh one.
 */
export declare class RendezvousIdentity {
  secretKey: Uint8Array
  publicKey: Uint8Array
  /** canonical base64url public key — the address of this identity */
  key: string

  /** @param secretKey - 32-byte Ed25519 seed/secret */
  constructor(secretKey: Uint8Array | ArrayBufferLike)

  static generate(): RendezvousIdentity

  /** Sign a canonical message; returns the signature as base64url. */
  sign(msg: Uint8Array): string
}

/** Fresh random nonce (base64url of 16 bytes) for replay protection. */
export declare function randomNonce(): string

export interface AnnounceResult {
  ok: boolean
  key: string
  ttl: number
  expires_at: number
}

export interface ResolveResult {
  key: string
  online: boolean
  endpoints?: string[]
  meta?: string
  expires_at?: number
}

export interface DepositResult {
  ok: boolean
  id: string
  expires_at: number
}

/** One decoded blob from a signal/mailbox poll — the opaque `payload` bytes are pre-decoded. */
export interface PolledBlob {
  id: string
  from: string
  /** decoded opaque bytes */
  payload: Uint8Array
  payloadB64: string
  ts: number
  exp: number
}

export interface AckResult {
  deleted: number
}

export interface IceServer {
  urls: string[]
  username?: string
  credential?: string
  ttl?: number
}

/** Constructor options for {@link RendezvousClient}. */
export interface RendezvousClientOptions {
  /** relay origin, e.g. "https://relay.example.com" */
  baseUrl: string
  /** this peer's Ed25519 identity */
  identity: RendezvousIdentity
  /** mount prefix (default "/rendezvous") */
  prefix?: string
  /** optional Bearer token for the relay's own gate (refused on an insecure baseUrl) */
  authToken?: string | null
  /** injectable fetch (tests) */
  fetch?: typeof fetch
}

/** RendezvousClient talks to a single relayd's rendezvous surface. */
export declare class RendezvousClient {
  baseUrl: string
  prefix: string
  identity: RendezvousIdentity
  /** this identity's rendezvous address (Ed25519 public key, base64url) */
  key: string

  constructor(opts: RendezvousClientOptions)

  /**
   * Return a new RendezvousClient that talks to the SAME relay under a
   * DIFFERENT Ed25519 identity.
   */
  withIdentity(identity: RendezvousIdentity): RendezvousClient

  /** Announce this identity's presence (signed, TTL'd). */
  announce(opts?: { endpoints?: string[], meta?: string, ttl?: number }): Promise<AnnounceResult>

  /** Withdraw this identity's presence record (signed). */
  withdraw(): Promise<{ ok: boolean }>

  /** Resolve a key to its current presence. Unauthenticated read. */
  resolve(key: string): Promise<ResolveResult>

  /** Deposit an opaque WebRTC signal blob addressed to recipientKey. */
  signalDeposit(recipientKey: string, payload: Uint8Array | string, ttl?: number): Promise<DepositResult>

  /** Long-poll this identity's signal inbox. */
  signalPoll(opts?: { wait?: number }): Promise<PolledBlob[]>

  /** Ack (delete) consumed signal blobs by id. */
  signalAck(ids: string | string[]): Promise<AckResult>

  /** Deposit an opaque encrypted blob into recipientKey's mailbox. */
  mailboxDeposit(recipientKey: string, payload: Uint8Array | string, ttl?: number): Promise<DepositResult>

  /** Long-poll this identity's mailbox. */
  mailboxPoll(opts?: { wait?: number }): Promise<PolledBlob[]>

  /** Ack (delete) consumed mailbox blobs by id. */
  mailboxAck(ids: string | string[]): Promise<AckResult>

  /** Fetch the relay's ICE server list (STUN + ephemeral-cred TURN). */
  ice(): Promise<IceServer[]>

  /** The ICE URL (for handing to FabricClient's iceUrl option). */
  iceUrl(): string
}

export declare const RENDEZVOUS_DOMAINS: {
  announce: string
  withdraw: string
  signalDeposit: string
  signalPoll: string
  signalAck: string
  mailboxDeposit: string
  mailboxPoll: string
  mailboxAck: string
}
