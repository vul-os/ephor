/**
 * fabric.dos.test.js — DoS / resource-exhaustion protection (MED audit fix #5)
 *
 * Covers:
 *   • Oversized relay blobs are dropped before dispatch (relay blob size cap)
 *   • Relay blob signature required when sender's key is known (inbound auth, MED #3)
 *   • pendingCandidates buffer is capped at MAX_PENDING_CANDIDATES (50)
 *   • _peers map is capped at MAX_PEERS (50)
 *   • Oversized data-channel payloads are dropped (data-channel size cap)
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { FabricClient, type FabricClientOptions, type FabricMessageDetail } from '../fabric.js'
import { SignalingClient } from '../signaling.js'
import { makeRelayBlob } from './_relayTestUtil.js'

interface DepositForm {
  to: string
  from: string
  nonce: string
  blob_b64: string
  epk: string
}

// Sign the relay-deposit form { to, from, nonce, blob_b64, epk } with an
// ECDSA P-256 private key and return a base64 signature.
async function signDepositForm(privateKey: CryptoKey, { to, from, nonce, blob_b64, epk }: DepositForm): Promise<string> {
  const sigMsg = JSON.stringify({ to, from, nonce, blob_b64, epk })
  const sigBuf = await crypto.subtle.sign(
    { name: 'ECDSA', hash: 'SHA-256' },
    privateKey,
    new TextEncoder().encode(sigMsg),
  )
  return btoa(String.fromCharCode(...new Uint8Array(sigBuf)))
}

// ── Fake WebSocket ─────────────────────────────────────────────────────────────

class FakeWebSocket {
  static OPEN = 1
  static CONNECTING = 0
  static CLOSED = 3
  static instances: FakeWebSocket[] = []
  static last: FakeWebSocket | null = null

  url: string
  protocols: string[]
  readyState: number
  sent: string[]
  _listeners: Record<string, Array<(payload: unknown) => void>>

  constructor(url: string, protocols?: string[]) {
    this.url = url
    this.protocols = protocols || []
    this.readyState = FakeWebSocket.CONNECTING
    this.sent = []
    this._listeners = {}
    FakeWebSocket.instances.push(this)
    FakeWebSocket.last = this
  }

  addEventListener(evt: string, fn: (payload: unknown) => void) {
    if (!this._listeners[evt]) this._listeners[evt] = []
    this._listeners[evt].push(fn)
  }

  send(data: string) { this.sent.push(data) }
  close() { this.readyState = FakeWebSocket.CLOSED; this._fire('close', {}) }

  _fire(evt: string, payload: unknown) { for (const fn of (this._listeners[evt] || [])) fn(payload) }
  _open() { this.readyState = FakeWebSocket.OPEN; this._fire('open', {}) }
  _message(frame: unknown) {
    this._fire('message', { data: typeof frame === 'string' ? frame : JSON.stringify(frame) })
  }
}

// ── Fake RTCPeerConnection with event-capable DataChannel ─────────────────────

class FakeDataChannel {
  readyState: string
  binaryType: string
  sent: string[]
  _listeners: Record<string, Array<(payload: unknown) => void>>

  constructor() {
    this.readyState = 'connecting'
    this.binaryType = 'arraybuffer'
    this.sent = []
    this._listeners = {}
  }
  addEventListener(evt: string, fn: (payload: unknown) => void) {
    if (!this._listeners[evt]) this._listeners[evt] = []
    this._listeners[evt].push(fn)
  }
  _fire(evt: string, payload: unknown) { for (const fn of (this._listeners[evt] || [])) fn(payload) }
  send(d: string) { this.sent.push(d) }
  close() {}
}

interface FakeSessionDescription {
  type: string
  sdp: string
}

class FakePC {
  static instances: FakePC[] = []
  static last: FakePC | null = null

  _listeners: Record<string, Array<(payload: unknown) => void>>
  connectionState: string
  localDescription: FakeSessionDescription | null
  remoteDescription: FakeSessionDescription | null
  _dc: FakeDataChannel

  constructor() {
    this._listeners = {}
    this.connectionState = 'connecting'
    this.localDescription = null
    this.remoteDescription = null
    this._dc = new FakeDataChannel()
    FakePC.instances.push(this)
    FakePC.last = this
  }

  addEventListener(evt: string, fn: (payload: unknown) => void) {
    if (!this._listeners[evt]) this._listeners[evt] = []
    this._listeners[evt].push(fn)
  }
  _fire(evt: string, payload: unknown) { for (const fn of (this._listeners[evt] || [])) fn(payload) }

  createOffer()  { return Promise.resolve({ type: 'offer',  sdp: 'v=0 fake' }) }
  createAnswer() { return Promise.resolve({ type: 'answer', sdp: 'v=0 fake' }) }
  setLocalDescription(d: FakeSessionDescription)  { this.localDescription  = d; return Promise.resolve() }
  setRemoteDescription(d: FakeSessionDescription) { this.remoteDescription = d; return Promise.resolve() }
  addIceCandidate()       { return Promise.resolve() }
  close() { this.connectionState = 'closed' }
  createDataChannel() { return this._dc }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeFabric(peerId = 'local', sessionId = 'sess-1', opts: Partial<FabricClientOptions> = {}) {
  return new FabricClient({
    sessionId,
    peerId,
    signalingUrl: 'ws://localhost/sig',
    iceUrl: '/api/peering/ice',
    relayBaseUrl: '',
    ...opts,
  })
}

function sleep(ms: number) { return new Promise(r => setTimeout(r, ms)) }

// ── Setup / teardown ──────────────────────────────────────────────────────────

beforeEach(() => {
  FakeWebSocket.instances = []
  FakeWebSocket.last = null
  FakePC.instances = []
  FakePC.last = null

  vi.stubGlobal('WebSocket', FakeWebSocket)
  vi.stubGlobal('RTCPeerConnection', FakePC)
  vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, json: async () => ({ ice_servers: [] }) })))
  vi.spyOn(console, 'warn').mockImplementation(() => {})
  vi.spyOn(console, 'info').mockImplementation(() => {})
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => { vi.restoreAllMocks() })

// ── 1. Relay blob size cap ─────────────────────────────────────────────────────

describe('FabricClient relay — oversized blob dropped', () => {
  it('drops a relay blob whose decoded payload exceeds MAX_PAYLOAD_BYTES (256 KB)', async () => {
    const fc = makeFabric()
    await fc._ensureDepositKey()

    // Build an ENCRYPTED payload whose plaintext JSON is just over 256 KB.
    const bigData = 'x'.repeat(256 * 1024 + 1)
    const { blob_b64, epk } = makeRelayBlob({
      recipientBoxPubB64: fc._boxPubKeyB64!, to: 'local', from: 'remote',
      session: 'sess-1', data: bigData,
    })

    vi.stubGlobal('fetch', vi.fn(async (url: string | URL) => {
      if (String(url).includes('pickup')) {
        return { ok: true, json: async () => ({ blobs: [{ id: 'b1', from: 'remote', blob_b64, epk }] }) }
      }
      return { ok: true, json: async () => ({ ice_servers: [] }) }
    }))

    const received: FabricMessageDetail[] = []
    fc.addEventListener('message', (ev) => received.push((ev as CustomEvent<FabricMessageDetail>).detail))

    fc._peers.set('remote', {
      id: 'remote', state: 'relay', dc: null, pc: null,
      relayTimer: null, pendingCandidates: [], reset() {},
      reinitTimer: null, reinitDelay: 0, left: false,
    })

    await fc._relayPoll()

    // Oversized blob must be silently dropped — no message dispatched
    expect(received).toHaveLength(0)
    fc.leave()
  })

  it('dispatches a relay blob that is exactly at the limit (256 KB)', async () => {
    const fc = makeFabric()
    await fc._ensureDepositKey()

    // MAX_PAYLOAD_BYTES = 256 * 1024 = 262144. The post-decrypt cap is on the
    // decrypted JSON string length. Build a payload that decrypts to exactly that.
    const overhead = JSON.stringify({ session: 'sess-1', data: '' }).length
    const data = 'y'.repeat(256 * 1024 - overhead)
    const { blob_b64, epk } = makeRelayBlob({
      recipientBoxPubB64: fc._boxPubKeyB64!, to: 'local', from: 'remote',
      session: 'sess-1', data,
    })

    vi.stubGlobal('fetch', vi.fn(async (url: string | URL) => {
      if (String(url).includes('pickup')) {
        return { ok: true, json: async () => ({ blobs: [{ id: 'b2', from: 'remote', blob_b64, epk }] }) }
      }
      return { ok: true, json: async () => ({ ice_servers: [] }) }
    }))

    const received: FabricMessageDetail[] = []
    fc.addEventListener('message', (ev) => received.push((ev as CustomEvent<FabricMessageDetail>).detail))

    fc._peers.set('remote', {
      id: 'remote', state: 'relay', dc: null, pc: null,
      relayTimer: null, pendingCandidates: [], reset() {},
      reinitTimer: null, reinitDelay: 0, left: false,
    })

    await fc._relayPoll()

    expect(received).toHaveLength(1)
    expect(received[0]!.data).toBe(data)
    fc.leave()
  })
})

// ── 2. Relay inbound signature verification (MED audit fix #3) ────────────────

describe('FabricClient relay — inbound blob signature check', () => {
  // Register `remote`'s ECDSA verify key directly in the signaling registry,
  // mirroring a prior keyed 'join', without the WS dance.
  async function registerRemoteKey(fc: FabricClient, keyPair: CryptoKeyPair) {
    const rawPub = await crypto.subtle.exportKey('raw', keyPair.publicKey)
    const pubKeyB64 = btoa(String.fromCharCode(...new Uint8Array(rawPub)))
    const verifyKey = await crypto.subtle.importKey(
      'raw', Uint8Array.from(atob(pubKeyB64), c => c.charCodeAt(0)),
      { name: 'ECDSA', namedCurve: 'P-256' }, false, ['verify'],
    )
    ;(fc._signaling as SignalingClient)._peerKeys.set('remote', verifyKey)
  }

  it('drops an (encrypted) blob from a known-keyed sender that has no sig', async () => {
    const keyPair = await crypto.subtle.generateKey(
      { name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign', 'verify'],
    ) as CryptoKeyPair

    const fc = makeFabric()
    await fc._ensureDepositKey()
    await registerRemoteKey(fc, keyPair)
    expect(fc._signaling.hasPeerKey('remote')).toBe(true)

    // Encrypted blob (carries epk) but unsigned, from a known-keyed sender.
    const { blob_b64, epk } = makeRelayBlob({
      recipientBoxPubB64: fc._boxPubKeyB64!, to: 'local', from: 'remote',
      session: 'sess-1', data: 'should-be-dropped',
    })

    vi.stubGlobal('fetch', vi.fn(async (url: string | URL) => {
      if (String(url).includes('pickup')) {
        return { ok: true, json: async () => ({ blobs: [{ id: 'b3', from: 'remote', blob_b64, epk }] }) }
      }
      return { ok: true, json: async () => ({ ice_servers: [] }) }
    }))

    const received: FabricMessageDetail[] = []
    fc.addEventListener('message', (ev) => received.push((ev as CustomEvent<FabricMessageDetail>).detail))

    fc._peers.set('remote', {
      id: 'remote', state: 'relay', dc: null, pc: null,
      relayTimer: null, pendingCandidates: [], reset() {},
      reinitTimer: null, reinitDelay: 0, left: false,
    })

    await fc._relayPoll()

    // No sig but sender's key is known → must be dropped
    expect(received).toHaveLength(0)
    fc.leave()
  })

  it('drops a blob from a known-keyed sender with an invalid sig', async () => {
    const keyPair = await crypto.subtle.generateKey(
      { name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign', 'verify'],
    ) as CryptoKeyPair
    const wrongKeyPair = await crypto.subtle.generateKey(
      { name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign', 'verify'],
    ) as CryptoKeyPair

    const fc = makeFabric()
    await fc._ensureDepositKey()
    await registerRemoteKey(fc, keyPair)

    const { blob_b64, epk } = makeRelayBlob({
      recipientBoxPubB64: fc._boxPubKeyB64!, to: 'local', from: 'remote',
      session: 'sess-1', data: 'tampered',
    })
    const nonce = crypto.randomUUID()
    // Sign the deposit form with a WRONG key (mallory impersonates 'remote')
    const sig = await signDepositForm(wrongKeyPair.privateKey, {
      to: 'local', from: 'remote', nonce, blob_b64, epk,
    })

    vi.stubGlobal('fetch', vi.fn(async (url: string | URL) => {
      if (String(url).includes('pickup')) {
        return { ok: true, json: async () => ({ blobs: [{ id: 'b4', from: 'remote', blob_b64, nonce, sig, epk }] }) }
      }
      return { ok: true, json: async () => ({ ice_servers: [] }) }
    }))

    const received: FabricMessageDetail[] = []
    fc.addEventListener('message', (ev) => received.push((ev as CustomEvent<FabricMessageDetail>).detail))

    fc._peers.set('remote', {
      id: 'remote', state: 'relay', dc: null, pc: null,
      relayTimer: null, pendingCandidates: [], reset() {},
      reinitTimer: null, reinitDelay: 0, left: false,
    })

    await fc._relayPoll()

    expect(received).toHaveLength(0)
    fc.leave()
  })

  it('accepts a blob from a known-keyed sender with a valid sig', async () => {
    const keyPair = await crypto.subtle.generateKey(
      { name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign', 'verify'],
    ) as CryptoKeyPair

    const fc = makeFabric()
    await fc._ensureDepositKey()
    await registerRemoteKey(fc, keyPair)

    const { blob_b64, epk } = makeRelayBlob({
      recipientBoxPubB64: fc._boxPubKeyB64!, to: 'local', from: 'remote',
      session: 'sess-1', data: 'valid-signed-payload',
    })
    const nonce = crypto.randomUUID()
    // Correct signing of { to, from, nonce, blob_b64, epk }
    const sig = await signDepositForm(keyPair.privateKey, {
      to: 'local', from: 'remote', nonce, blob_b64, epk,
    })

    vi.stubGlobal('fetch', vi.fn(async (url: string | URL) => {
      if (String(url).includes('pickup')) {
        return { ok: true, json: async () => ({ blobs: [{ id: 'b5', from: 'remote', blob_b64, nonce, sig, epk }] }) }
      }
      return { ok: true, json: async () => ({ ice_servers: [] }) }
    }))

    const received: FabricMessageDetail[] = []
    fc.addEventListener('message', (ev) => received.push((ev as CustomEvent<FabricMessageDetail>).detail))

    fc._peers.set('remote', {
      id: 'remote', state: 'relay', dc: null, pc: null,
      relayTimer: null, pendingCandidates: [], reset() {},
      reinitTimer: null, reinitDelay: 0, left: false,
    })

    await fc._relayPoll()

    expect(received).toHaveLength(1)
    expect(received[0]!.data).toBe('valid-signed-payload')
    fc.leave()
  })

  it('allows an (encrypted) unsigned blob from an unknown sender (backward compat — no key stored)', async () => {
    const fc = makeFabric()
    await fc._ensureDepositKey()

    // Unknown sender (no ECDSA key stored): still encrypted (carries epk).
    // Confidentiality holds regardless of authenticity; the blob is accepted.
    const { blob_b64, epk } = makeRelayBlob({
      recipientBoxPubB64: fc._boxPubKeyB64!, to: 'local', from: 'stranger',
      session: 'sess-1', data: 'unknown-sender-no-key',
    })

    vi.stubGlobal('fetch', vi.fn(async (url: string | URL) => {
      if (String(url).includes('pickup')) {
        return { ok: true, json: async () => ({ blobs: [{ id: 'b6', from: 'stranger', blob_b64, epk }] }) }
      }
      return { ok: true, json: async () => ({ ice_servers: [] }) }
    }))

    const received: FabricMessageDetail[] = []
    fc.addEventListener('message', (ev) => received.push((ev as CustomEvent<FabricMessageDetail>).detail))

    fc._peers.set('stranger', {
      id: 'stranger', state: 'relay', dc: null, pc: null,
      relayTimer: null, pendingCandidates: [], reset() {},
      reinitTimer: null, reinitDelay: 0, left: false,
    })

    // No key stored for 'stranger' → hasPeerKey('stranger') === false → allow through
    await fc._relayPoll()

    expect(received).toHaveLength(1)
    expect(received[0]!.data).toBe('unknown-sender-no-key')
    fc.leave()
  })
})

// ── 3. pendingCandidates cap ──────────────────────────────────────────────────

describe('FabricClient — pendingCandidates buffer capped at 50', () => {
  it('never buffers more than MAX_PENDING_CANDIDATES ICE candidates per peer', async () => {
    // Use requirePeerAuth=false so unsigned ICE frames from alice are allowed through
    // (this tests the DoS cap in isolation, independent of the auth path).
    const fc = makeFabric('bob', 'sess-1', { requirePeerAuth: false })
    await fc.join()
    const ws = FakeWebSocket.last!
    ws._open()

    // alice's join — creates a PeerState but bob (polite: b > a) doesn't offer
    ws._message({
      channel: 'signal',
      from: 'alice',
      payload: { type: 'join', session: 'sess-1' },
    })
    await sleep(20)

    // alice sends 60 ICE candidates; no offer/answer yet so they queue
    for (let i = 0; i < 60; i++) {
      ws._message({
        channel: 'signal',
        from: 'alice',
        payload: {
          type: 'ice', session: 'sess-1', to: 'bob',
          candidate: { candidate: `candidate:${i} 1 udp 12345 10.0.0.1 5000${i} typ host`, sdpMid: '0' },
        },
      })
    }
    await sleep(50)

    const peerState = fc._peers.get('alice')
    expect(peerState).toBeTruthy()
    // Must be capped at MAX_PENDING_CANDIDATES (50) regardless of how many arrived
    expect(peerState!.pendingCandidates.length).toBeLessThanOrEqual(50)

    fc.leave()
  })
})

// ── 4. Peer map cap ───────────────────────────────────────────────────────────

describe('FabricClient — _peers map capped at MAX_PEERS (50)', () => {
  it('stops accepting new peers once the cap is reached', async () => {
    const fc = makeFabric('z-local', 'sess-1', { requirePeerAuth: false })
    await fc.join()
    const ws = FakeWebSocket.last!
    ws._open()

    // Flood 60 join signals from different peers
    for (let i = 0; i < 60; i++) {
      ws._message({
        channel: 'signal',
        from: `peer-${i}`,
        payload: { type: 'join', session: 'sess-1' },
      })
    }
    await sleep(100)

    // z-local > peer-N lexicographically → z-local is impolite and sends offers.
    // _getOrCreatePeer caps at MAX_PEERS (50); the excess are silently dropped.
    expect(fc._peers.size).toBeLessThanOrEqual(50)

    fc.leave()
  })
})

// ── 5. Data-channel payload size cap ─────────────────────────────────────────

describe('FabricClient — data-channel oversized payload dropped', () => {
  it('drops a data-channel message exceeding MAX_PAYLOAD_BYTES and does not dispatch it', async () => {
    const fc = makeFabric('bob', 'sess-1', { requirePeerAuth: false })

    const received: FabricMessageDetail[] = []
    fc.addEventListener('message', (ev) => received.push((ev as CustomEvent<FabricMessageDetail>).detail))

    // Wire a fake data channel directly so we can fire message events
    const fdc = new FakeDataChannel()
    const fakePeerState = {
      id: 'alice', state: 'connected' as const, dc: fdc as unknown as RTCDataChannel, pc: null,
      relayTimer: null, pendingCandidates: [] as RTCIceCandidateInit[], reset() {},
      reinitTimer: null, reinitDelay: 0, left: false,
    }
    fc._peers.set('alice', fakePeerState)

    // Patch _wireDataChannel to set up listeners on fdc
    fc._wireDataChannel(fdc as unknown as RTCDataChannel, 'alice', fakePeerState)

    // Fire an oversized payload (> 256 KB)
    const oversized = 'x'.repeat(256 * 1024 + 1)
    fdc._fire('message', { data: oversized })

    expect(received).toHaveLength(0)   // oversized payload must be dropped
    expect(console.warn).toHaveBeenCalledWith(
      expect.stringContaining('[fabric]'),
      expect.anything(),
      expect.anything(),
    )

    fc.leave()
  })

  it('dispatches a data-channel message that is within the size limit', async () => {
    const fc = makeFabric('bob', 'sess-1', { requirePeerAuth: false })

    const received: FabricMessageDetail[] = []
    fc.addEventListener('message', (ev) => received.push((ev as CustomEvent<FabricMessageDetail>).detail))

    const fdc = new FakeDataChannel()
    const fakePeerState = {
      id: 'alice', state: 'connected' as const, dc: fdc as unknown as RTCDataChannel, pc: null,
      relayTimer: null, pendingCandidates: [] as RTCIceCandidateInit[], reset() {},
      reinitTimer: null, reinitDelay: 0, left: false,
    }
    fc._peers.set('alice', fakePeerState)
    fc._wireDataChannel(fdc as unknown as RTCDataChannel, 'alice', fakePeerState)

    const payload = 'hello from alice'
    fdc._fire('message', { data: payload })

    expect(received).toHaveLength(1)
    expect(received[0]!.data).toBe(payload)
    expect(received[0]!.from).toBe('alice')

    fc.leave()
  })
})
