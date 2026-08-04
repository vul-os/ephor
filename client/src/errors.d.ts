// errors.d.ts — hand-written type declarations for the pinned kotva-client
// substrate module errors.js (see kotva-client.pin.json).
//
// errors.js is BYTE-IDENTICAL to kotva-client@0.1.0's bindings/js/src/errors.js
// and MUST NOT be edited in this repo (scripts/check-kotva-parity.mjs enforces
// this with a sha256 pin). This declaration file gives TypeScript consumers in
// this repo real types for it without touching the pinned source: TypeScript
// resolves a same-basename `.d.ts` ahead of the `.js` file for type-checking
// purposes, while `.js` remains the actual runtime module.
//
// Keep this in sync with errors.js by hand if kotva ever re-pins a new tag —
// this file carries no independent authority over the runtime behaviour.

export interface SignalingErrorDetail {
  code?: string
  attempts?: number
}

/**
 * Thrown when the signaling transport cannot be established or is permanently
 * unavailable (e.g. budget exhausted and no recovery path).
 */
export declare class SignalingError extends Error {
  code: string
  attempts: number | undefined
  constructor(message: string, detail?: SignalingErrorDetail)
}

export interface RelayDepositErrorDetail {
  code?: string
  status?: number
}

/**
 * Thrown when a relay deposit cannot be completed (network failure, server
 * rejection, or a signing failure).
 */
export declare class RelayDepositError extends Error {
  code: string
  /** HTTP status when available */
  status: number | undefined
  constructor(message: string, detail?: RelayDepositErrorDetail)
}

export interface EndpointErrorDetail {
  code?: string
  candidates?: string[]
}

/**
 * Thrown when no reachable endpoint can be found after probing all candidates.
 */
export declare class EndpointError extends Error {
  code: string
  candidates: string[] | undefined
  constructor(message: string, detail?: EndpointErrorDetail)
}

export interface FabricErrorDetail {
  code?: string
  peerId?: string
}

/**
 * Thrown when a fabric session operation fails (e.g. ICE negotiation fatal
 * error, or the P2P and relay paths both fail).
 */
export declare class FabricError extends Error {
  code: string
  peerId: string | undefined
  constructor(message: string, detail?: FabricErrorDetail)
}
