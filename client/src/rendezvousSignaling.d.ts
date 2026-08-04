// rendezvousSignaling.d.ts — hand-written type declarations for the pinned
// kotva-client substrate module rendezvousSignaling.js (see kotva-client.pin.json).
// See errors.d.ts for why this file exists and the rule it follows.
//
// rendezvousSignaling.js is a drop-in signaling transport for FabricClient
// that runs the complete WebRTC signaling lifecycle over any vulos-relayd's
// OPEN rendezvous surface, with no host box. It presents the SAME interface
// FabricClient already consumes from SignalingClient — see its module
// docstring for the full identity-bridging and discovery design.

import type { SignFrameFn, SignedPreKeyAnnouncement, SignalPayload } from './signaling.js'
import type { RendezvousClient, RendezvousIdentity } from './rendezvous.js'

/**
 * Derive the deterministic Ed25519 room identity for a sessionId. Every
 * member of the session computes the identical keypair, so its rendezvous
 * inbox is a shared discovery board. Not secret — knowing the sessionId is
 * what grants membership.
 */
export declare function deriveRoomIdentity(sessionId: string): RendezvousIdentity

/** Constructor options for {@link RendezvousSignalingClient}. */
export interface RendezvousSignalingClientOptions {
  /**
   * the caller's per-peer rendezvous client (its Ed25519 identity is this
   * peer's rendezvous address). The room client is derived from it via
   * withIdentity().
   */
  selfClient: RendezvousClient
  sessionId: string
  peerId: string
  getDepositPubKey?: (() => string | null) | null
  getBoxPubKey?: (() => string | null) | null
  getSignedPreKey?: (() => SignedPreKeyAnnouncement | null) | null
  signFrame?: SignFrameFn | null
  requirePeerAuth?: boolean
  maxAttempts?: number
  /**
   * start the recurring long-poll timers on connect(). Tests set false and
   * drive `_pollBoardOnce`/`_pollInboxOnce`.
   */
  pollLoop?: boolean
}

/**
 * A drop-in signaling transport for FabricClient that runs the complete
 * WebRTC signaling lifecycle (presence discovery + offer/answer/ICE exchange
 * + polite-peer negotiation) over any vulos-relayd's OPEN rendezvous surface,
 * with no host box and no /api/peering/* backend.
 */
export declare class RendezvousSignalingClient extends EventTarget {
  constructor(opts: RendezvousSignalingClientOptions)

  /** This peer's rendezvous address (Ed25519 public key, base64url). */
  readonly key: string

  /** The shared session room's rendezvous address. */
  readonly roomKey: string

  /** The rendezvous address learned for a peerId, or null. */
  rdvKeyFor(peerId: string): string | null

  /**
   * Announce presence, deposit our join onto the session board, and start
   * the discovery + inbox long-poll loops.
   */
  connect(): Promise<void>

  /** Cleanly leave: tombstone on the board, withdraw presence, stop loops. */
  close(): void

  /** Send a signal (offer/answer/ice) to a peer over its rendezvous inbox. */
  signal(type: string, toId: string | null, data?: Partial<SignalPayload>): Promise<void>

  hasPeerKey(peerId: string): boolean
  getPeerBoxKey(peerId: string): string | null
  getPeerSignedPreKey(peerId: string): SignedPreKeyAnnouncement | null
  isPeerV2Capable(peerId: string): boolean
  verifyPeerSignedPreKey(peerId: string, signedPreKey: { pub: string, sig: string }): Promise<boolean>
  verifyPeerSig(peerId: string, message: string, sigB64: string): Promise<boolean | null>

  // ── internal, exposed for tests (see rendezvousSignaling.test.js) ──────────
  _pollBoardOnce(wait?: number): Promise<number>
  _pollInboxOnce(wait?: number): Promise<number>
}
