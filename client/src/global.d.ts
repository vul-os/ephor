// global.d.ts — ambient window injection point shared by endpoints.ts,
// call/ice.ts and call/fabricSignaling.ts.
//
// The host application (OS shell / office suite) may set
// `window.__VULOS_ENDPOINTS__` before this SDK loads to inject discovery
// config that would otherwise require a network round-trip or a build-time
// env var. Every field is optional — none of these modules require it.

export interface VulosEndpointsInjection {
  /** Cloud-routed base URL, e.g. "https://<box>.vulos.org". */
  cloud?: string
  /** LAN-direct base URL, e.g. "https://box.<id>.lan.vulos.org". */
  lan?: string
  /** Peering signaling WebSocket URL (see call/fabricSignaling.ts). */
  signalingUrl?: string
  /** Opt in to the well-known Google public STUN server as an ICE fallback. */
  googleStunFallback?: boolean
  /** Explicit ICE server fallback list (see call/ice.ts). */
  iceServersFallback?: RTCIceServer[]
}

declare global {
  interface Window {
    __VULOS_ENDPOINTS__?: VulosEndpointsInjection
  }
}
