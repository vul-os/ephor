// secureTransport.d.ts — hand-written type declarations for the pinned
// kotva-client substrate module secureTransport.js (see kotva-client.pin.json).
// See errors.d.ts for why this file exists and the rule it follows.

/**
 * True when `hostname` is a loopback address where plaintext is acceptable for
 * local development (the token never leaves the machine).
 *
 * @param hostname  URL.hostname (IPv6 keeps its surrounding brackets)
 */
export declare function isLoopbackHost(hostname: string): boolean

/**
 * Decide whether a credential (Bearer JWT / WS token) may be attached to a
 * request bound for `rawUrl`. Fail-closed: unknown / unparseable / plaintext-
 * remote URLs return false so the caller refuses to leak the token.
 *
 * @param rawUrl  absolute URL, or '' / a relative path for same-origin
 */
export declare function tokenTransportSecure(rawUrl: string | null | undefined): boolean
