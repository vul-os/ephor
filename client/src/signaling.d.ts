// signaling.d.ts — hand-written type declarations for the pinned kotva-client
// substrate module signaling.js (see kotva-client.pin.json).
// See errors.d.ts for why this file exists and the rule it follows.
//
// signaling.js is the transport-agnostic E2E peer-authentication core shared by
// the WebSocket path (this file) and the rendezvous transport
// (rendezvousSignaling.js, via its `_core`). Some of the shapes below are
// deliberately WIDER than kotva's own bindings/js/src/signaling.ts:
//   • `type` on SignalPayload / signal() is `string`, not the literal
//     'offer'|'answer'|'ice'|'join'|'leave' union — the call layer
//     (call/fabricSignaling.ts) reuses this exact envelope for arbitrary
//     app-level kinds ('sdp', 'screen-share', ...), and the runtime code only
//     ever does `p.type === 'join'` string comparisons, never a switch that
//     would reject an unknown kind.
//   • SignalPayload carries two additive fields, `data` and `identity`, that
//     kotva's core protocol does not define but the call layer spreads through
//     the same envelope (`{ ...data }` in _buildSignalPayload) unchanged.

/** The signed prekey {id, pub, sig} as announced/claimed on the wire (base64 fields). */
export interface SignedPreKeyAnnouncement {
  id: string
  pub: string
  sig: string
}

/** A SignalPayload as it appears on the wire (untrusted until verified — see _processSignal). */
export interface SignalPayload {
  /** 'offer' | 'answer' | 'ice' | 'join' | 'leave', or an app-defined kind carried by the call layer */
  type: string
  session?: string
  /** targeted delivery (optional; omit/null = broadcast) */
  to?: string | null
  /** offer / answer SDP */
  sdp?: string
  candidate?: RTCIceCandidateInit
  /** replay-protection nonce (required when sig present) */
  nonce?: string
  /** signed epoch-ms timestamp (required when sig present) */
  ts?: number
  /** base64 ECDSA P-256 signature over the canonical payload */
  sig?: string
  /** sender's raw ECDSA public key, base64 (offer/answer only) */
  pubKey?: string | null
  /** published on 'join': base64 raw ECDSA P-256 deposit-signing public key */
  depositPubKey?: string | null
  /** published on 'join': base64 raw X25519 box (encryption) public key */
  boxPubKey?: string | null
  /** published on 'join': this peer's signed prekey for the v2 (X3DH) relay path */
  signedPreKey?: SignedPreKeyAnnouncement | null
  /** published on 'join': signed forward-secrecy capability commitment */
  supportsV2?: boolean
  /** call-layer extension: opaque app payload (sdp/ice/screen-share/etc. body) */
  data?: unknown
  /** call-layer extension: opaque caller identity carried alongside 'join' */
  identity?: unknown
}

/** Signs a canonical string with this client's ECDSA identity, returning a base64 signature. */
export type SignFrameFn = (msg: string) => Promise<string>

/** Constructor options for {@link SignalingClient}. */
export interface SignalingClientOptions {
  /** WebSocket URL, e.g. "ws://localhost:8080/api/peering/stream" */
  signalingUrl: string
  /** fabric session / document id */
  sessionId: string
  /** this client's identity token (injected by auth) */
  peerId: string
  /** Bearer JWT (if auth is enabled) */
  authToken?: string | null
  /**
   * how the auth JWT is delivered. 'subprotocol' (default) sends it in the
   * Sec-WebSocket-Protocol header; 'query' is a legacy migration shim.
   */
  tokenTransport?: 'subprotocol' | 'query'
  /** max reconnect attempts before 'offline' (default 10) */
  maxAttempts?: number
  /** returns this peer's base64 raw deposit signing public key, or null */
  getDepositPubKey?: (() => string | null) | null
  /** returns this peer's base64 raw X25519 box (encryption) public key, or null */
  getBoxPubKey?: (() => string | null) | null
  /** returns this peer's signed prekey for the v2 (X3DH) relay path, or null */
  getSignedPreKey?: (() => SignedPreKeyAnnouncement | null) | null
  /** signs a canonical string, returning a base64 ECDSA signature; null = unsigned frames */
  signFrame?: SignFrameFn | null
  /**
   * when true, offer/answer/ice frames from peers with no stored public key
   * are dropped. Frames from peers WITH a stored key are always verified
   * regardless of this flag.
   */
  requirePeerAuth?: boolean
}

/**
 * Authenticated signaling client — opens a WebSocket to a host signaling
 * stream and multiplexes offer/answer/ICE frames over the "signal" channel,
 * with E2E peer authentication (TOFU key/box/prekey import, signed-join
 * anti-downgrade pinning, offer/answer/ice signature + freshness + replay
 * checks). See the module docstring in signaling.js for the full protocol.
 */
export declare class SignalingClient extends EventTarget {
  constructor(opts: SignalingClientOptions)

  /** Connect (or reconnect) to the signaling WebSocket. */
  connect(): void

  /** Cleanly stop reconnecting and close the socket. */
  close(): void

  /**
   * Send a signal payload to a specific peer (or broadcast when `toId` is
   * null/omitted). `type` is any app-defined kind — 'offer'/'answer'/'ice' get
   * the E2E authentication treatment; other kinds ride the same signed
   * envelope unauthenticated at this layer (see call/fabricSignaling.ts).
   */
  signal(type: string, toId: string | null, data?: Partial<SignalPayload>): Promise<void>

  /** True when a public key has been stored for `fromPeerId` (join frame or TOFU import). */
  hasPeerKey(fromPeerId: string): boolean

  /** The stored base64 X25519 box (encryption) public key for `peerId`, or null. */
  getPeerBoxKey(peerId: string): string | null

  /** The stored, ECDSA-verified signed prekey for `peerId`, or null. */
  getPeerSignedPreKey(peerId: string): SignedPreKeyAnnouncement | null

  /** Whether `peerId` has been pinned as forward-secrecy (v2/X3DH) capable. */
  isPeerV2Capable(peerId: string): boolean

  /** Verify a signed prekey against the stored ECDSA key for `peerId`. Fails closed. */
  verifyPeerSignedPreKey(peerId: string, signedPreKey: { pub: string, sig: string }): Promise<boolean>

  /**
   * Verify a relay-deposit blob signature using the stored public key for
   * `fromPeerId`. Returns null when no key is stored (caller applies its own policy).
   */
  verifyPeerSig(fromPeerId: string, message: string, sigB64: string): Promise<boolean | null>

  // ── internal, but accessed cross-module by rendezvousSignaling.js, which
  // reuses this transport-agnostic core (its handshake pipeline and signed
  // payload builders) unconnected, feeding it frames read from a rendezvous
  // inbox instead of a WebSocket. Not part of the documented public API.

  /** Build (and, when signFrame is configured, sign) a SignalPayload. */
  _buildSignalPayload(type: string, toId: string, data?: Partial<SignalPayload>): Promise<SignalPayload>

  /** Build (and, when signFrame is configured, sign) this peer's 'join' announcement. */
  _buildJoinPayload(): Promise<SignalPayload>

  /** Run the full E2E peer-authentication pipeline on one inbound SignalPayload. */
  _processSignal(senderPeerId: string, p: SignalPayload): Promise<void>
}
