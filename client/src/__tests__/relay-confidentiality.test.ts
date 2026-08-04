/**
 * relay-confidentiality.test.ts — Fix 1 at the FabricClient layer.
 *
 * Asserts the end-to-end property the product markets: when collaboration falls
 * back to the relay circuit, the relay server only ever transports ciphertext,
 * and only the intended peer can read it.
 *
 *   1. TWO-PARTY ROUND-TRIP — alice deposits; the captured relay blob decrypts
 *      ONLY at bob (the addressed peer) and yields the original payload.
 *   2. THE RELAY SEES CIPHERTEXT — the deposited blob does not decode to the
 *      plaintext envelope.
 *   3. FAIL-CLOSED — if the recipient's box key is unknown, the deposit is
 *      SKIPPED (no plaintext is ever sent), not downgraded.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { FabricClient, type FabricMessageDetail } from '../fabric.js'
import type { SignalingClient } from '../signaling.js'

/** The signed+sealed deposit envelope POSTed to /api/peering/relay/deposit. */
interface DepositBody {
  to: string
  from: string
  blob_b64: string
  nonce: string
  sig: string
  epk: string
}

function makeFabric(peerId: string, sessionId = 'sess-1') {
  return new FabricClient({
    sessionId,
    peerId,
    signalingUrl: 'ws://localhost/sig',
    iceUrl: '/api/peering/ice',
    relayBaseUrl: '',
  })
}

beforeEach(() => {
  vi.spyOn(console, 'warn').mockImplementation(() => {})
  vi.spyOn(console, 'info').mockImplementation(() => {})
})
afterEach(() => { vi.restoreAllMocks() })

describe('Relay fallback — end-to-end confidentiality', () => {
  it('alice→relay→bob round-trips, and the blob decrypts only at bob', async () => {
    const alice = makeFabric('alice')
    const bob = makeFabric('bob')
    const eve = makeFabric('eve')   // an eavesdropper peer with its own box key
    await alice._ensureDepositKey()
    await bob._ensureDepositKey()
    await eve._ensureDepositKey()

    // Box-key exchange (as would happen via signaling 'join' frames).
    const aliceSig = alice._signaling as SignalingClient
    aliceSig._peerBoxKeys.set('bob', bob._boxPubKeyB64!)

    // Capture the deposit blob alice sends to the relay.
    let depositBody: DepositBody | undefined
    vi.stubGlobal('fetch', vi.fn(async (url: string | URL, opts?: RequestInit) => {
      if (String(url).includes('deposit')) depositBody = JSON.parse(String(opts?.body))
      return { ok: true, json: async () => ({ ice_servers: [] }) }
    }))

    await alice._relayDeposit('bob', { op: 'insert', text: 'hello world', pos: 42 })
    expect(depositBody).toBeTruthy()
    expect(depositBody!.to).toBe('bob')
    expect(depositBody!.epk).toBeTruthy()

    // Feed the SAME blob into bob's pickup → bob decrypts and dispatches.
    bob._peers.set('alice', {
      id: 'alice', state: 'relay', dc: null, pc: null,
      relayTimer: null, pendingCandidates: [], reset() {},
      reinitTimer: null, reinitDelay: 0, left: false,
    })
    const bobReceived: FabricMessageDetail[] = []
    bob.addEventListener('message', (ev) => bobReceived.push((ev as CustomEvent<FabricMessageDetail>).detail))
    vi.stubGlobal('fetch', vi.fn(async (url: string | URL) => {
      if (String(url).includes('pickup')) {
        return {
          ok: true,
          json: async () => ({ blobs: [{ id: 'x1', from: 'alice', blob_b64: depositBody!.blob_b64, epk: depositBody!.epk }] }),
        }
      }
      if (String(url).includes('ack')) return { ok: true }
      return { ok: true, json: async () => ({ ice_servers: [] }) }
    }))
    await bob._relayPoll()

    expect(bobReceived).toHaveLength(1)
    expect(bobReceived[0]!.from).toBe('alice')
    expect(bobReceived[0]!.data).toEqual({ op: 'insert', text: 'hello world', pos: 42 })

    // The SAME blob fed into eve's pickup → eve cannot decrypt → nothing delivered.
    eve._peers.set('alice', {
      id: 'alice', state: 'relay', dc: null, pc: null,
      relayTimer: null, pendingCandidates: [], reset() {},
      reinitTimer: null, reinitDelay: 0, left: false,
    })
    const eveReceived: FabricMessageDetail[] = []
    eve.addEventListener('message', (ev) => eveReceived.push((ev as CustomEvent<FabricMessageDetail>).detail))
    await eve._relayPoll()
    expect(eveReceived).toHaveLength(0)   // confidentiality holds vs other peers

    alice.leave(); bob.leave(); eve.leave()
  })

  it('the relay sees ciphertext, not the plaintext envelope', async () => {
    const alice = makeFabric('alice')
    const bob = makeFabric('bob')
    await alice._ensureDepositKey()
    await bob._ensureDepositKey()
    const aliceSig = alice._signaling as SignalingClient
    aliceSig._peerBoxKeys.set('bob', bob._boxPubKeyB64!)

    let depositBody: DepositBody | undefined
    vi.stubGlobal('fetch', vi.fn(async (url: string | URL, opts?: RequestInit) => {
      if (String(url).includes('deposit')) depositBody = JSON.parse(String(opts?.body))
      return { ok: true, json: async () => ({ ice_servers: [] }) }
    }))

    await alice._relayDeposit('bob', 'CONFIDENTIAL-DOC-BODY')

    const decodedBytes = atob(depositBody!.blob_b64)
    expect(decodedBytes).not.toContain('CONFIDENTIAL-DOC-BODY')
    expect(decodedBytes).not.toContain('session')
    let leaked = false
    try { JSON.parse(decodedBytes); leaked = true } catch { /* expected */ }
    expect(leaked).toBe(false)

    alice.leave(); bob.leave()
  })

  it('fails closed: no recipient box key → deposit is skipped, never sent in clear', async () => {
    const alice = makeFabric('alice')
    await alice._ensureDepositKey()
    // NOTE: no _peerBoxKeys entry for 'bob'.

    const depositCalls: DepositBody[] = []
    vi.stubGlobal('fetch', vi.fn(async (url: string | URL, opts?: RequestInit) => {
      if (String(url).includes('deposit')) depositCalls.push(JSON.parse(String(opts?.body)))
      return { ok: true, json: async () => ({ ice_servers: [] }) }
    }))

    await alice._relayDeposit('bob', 'must-not-leak')

    // No deposit was made at all — the payload is not sent in plaintext.
    expect(depositCalls).toHaveLength(0)
    // The outbound byte meter still counts the attempt (caller-visible).
    expect(alice.relayByteCount.out).toBeGreaterThan(0)

    alice.leave()
  })
})
