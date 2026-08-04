/**
 * presence.ts — Diwan Presence layer (OFFICE-24).
 *
 * Broadcasts {accountId, displayName, color, online} over a dedicated
 * "presence" channel on the OFFICE-20 FabricClient (separate from CRDT ops).
 *
 * Usage:
 *   const pm = new PresenceManager({ fabric, localIdentity })
 *   pm.addEventListener('roster', ({ detail: peers }) => …)
 *   pm.join()
 *   pm.leave()
 *
 * Identity resolution order:
 *   1. opts.localIdentity (caller-supplied, from Vulos account/vumail)
 *   2. localStorage "presence_identity" (persisted guest identity)
 *   3. Generated guest identity (random name + color, persisted)
 */

import type { FabricClient } from './fabric.js'

const PRESENCE_CHANNEL = 'presence'
const HEARTBEAT_MS = 10_000        // send heartbeat every 10 s
const TIMEOUT_MS = 25_000          // drop peer after 25 s of silence

// Valid status values for OFFICE-62
export const STATUS_ONLINE = 'online'
export const STATUS_AWAY   = 'away'
export const STATUS_DND    = 'dnd'
export const STATUS_IN_CALL = 'in-a-call'  // set by OFFICE-63 calling layer

export type PresenceStatus =
  | typeof STATUS_ONLINE
  | typeof STATUS_AWAY
  | typeof STATUS_DND
  | typeof STATUS_IN_CALL

/** Caller-supplied (or persisted guest) local identity. */
export interface LocalIdentity {
  accountId?: string
  displayName?: string
  isGuest?: boolean
}

interface ResolvedLocalIdentity {
  accountId: string
  displayName: string
  isGuest: boolean
}

/** The local user's presence record, as broadcast + surfaced via fullRoster. */
export interface LocalPresence {
  accountId: string
  displayName: string
  color: string
  online: boolean
  status: string
  statusText: string
  isGuest: boolean
  ts: number
}

/** A remote peer's presence record, as tracked in the roster. */
export interface RosterPeer {
  accountId: string
  displayName: string
  color: string
  online: boolean
  status: string
  statusText: string
  isGuest: boolean
  ts: number
  peerId: string
}

interface PresenceFramePayload {
  type?: 'join' | 'leave'
  accountId?: string
  displayName?: string
  color?: string
  status?: string
  statusText?: string
  isGuest?: boolean
}

interface PresenceFrame {
  channel: string
  payload: PresenceFramePayload
}

/** Deterministic color from a string (stable across sessions). */
function colorFromString(str: string): string {
  let hash = 0
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) - hash + str.charCodeAt(i)
    hash |= 0
  }
  const hue = Math.abs(hash) % 360
  return `hsl(${hue}, 65%, 50%)`
}

const GUEST_ADJECTIVES = ['Swift', 'Bright', 'Calm', 'Bold', 'Kind']
const GUEST_ANIMALS = ['Lemur', 'Falcon', 'Otter', 'Fox', 'Lynx']

function randomGuestName(): string {
  const adj = GUEST_ADJECTIVES[Math.floor(Math.random() * GUEST_ADJECTIVES.length)]
  const ani = GUEST_ANIMALS[Math.floor(Math.random() * GUEST_ANIMALS.length)]
  return `${adj} ${ani}`
}

function loadOrCreateLocalIdentity(): ResolvedLocalIdentity {
  try {
    const stored = localStorage.getItem('presence_identity')
    if (stored) {
      const parsed = JSON.parse(stored)
      if (parsed.accountId && parsed.displayName) return parsed
    }
  } catch { /* ignore */ }
  const identity: ResolvedLocalIdentity = {
    accountId: `guest:${crypto.randomUUID()}`,
    displayName: randomGuestName(),
    isGuest: true,
  }
  try { localStorage.setItem('presence_identity', JSON.stringify(identity)) } catch { /* ignore */ }
  return identity
}

/**
 * The structural surface PresenceManager needs from a fabric — satisfied by
 * FabricClient and by lightweight test doubles that only implement the
 * 'message'/'state' event pair + send(). Kept narrow deliberately (mirrors
 * fabric.ts's own SignalingTransport idiom) so tests don't need a full
 * FabricClient instance to exercise the presence layer in isolation.
 */
export interface PresenceFabricLike extends EventTarget {
  send(data: string): void
}

export interface PresenceManagerOptions {
  fabric: PresenceFabricLike
  /** Pass the Vulos account identity if authenticated; omit for guest. */
  localIdentity?: LocalIdentity | null
}

export class PresenceManager extends EventTarget {
  private _fabric: PresenceFabricLike
  private _local: LocalPresence
  private _roster: Map<string, RosterPeer>
  private _heartbeatTimer: ReturnType<typeof setInterval> | null
  private _gcTimer: ReturnType<typeof setInterval> | null
  private _stopped: boolean
  private _onFabricMessage: (ev: Event) => void
  private _onFabricState: (ev: Event) => void

  constructor({ fabric, localIdentity = null }: PresenceManagerOptions) {
    super()
    this._fabric = fabric

    const baseIdentity = (localIdentity || loadOrCreateLocalIdentity()) as ResolvedLocalIdentity
    this._local = {
      accountId: baseIdentity.accountId,
      displayName: baseIdentity.displayName,
      color: colorFromString(baseIdentity.accountId),
      online: true,
      status: STATUS_ONLINE,    // OFFICE-62: online | away | dnd | in-a-call
      statusText: '',           // OFFICE-62: free-text custom status
      isGuest: baseIdentity.isGuest ?? false,
      ts: Date.now(),
    }

    this._roster = new Map()
    this._heartbeatTimer = null
    this._gcTimer = null
    this._stopped = false

    // Listen for presence frames on the fabric message channel.
    this._onFabricMessage = (ev: Event) => this._handleMessage(ev as CustomEvent)
    this._fabric.addEventListener('message', this._onFabricMessage)

    // Also re-broadcast on new peer connections so late joiners see us immediately.
    this._onFabricState = (ev: Event) => this._handleState(ev as CustomEvent)
    this._fabric.addEventListener('state', this._onFabricState)
  }

  // ─── Public API ─────────────────────────────────────────────────────────────

  /** Start presence: broadcast join + begin heartbeat. */
  join(): void {
    this._broadcast()
    this._heartbeatTimer = setInterval(() => this._broadcast(), HEARTBEAT_MS)
    this._gcTimer = setInterval(() => this._gc(), HEARTBEAT_MS)
  }

  /**
   * OFFICE-62: Update local status and broadcast immediately.
   * @param status  - one of STATUS_ONLINE | STATUS_AWAY | STATUS_DND | STATUS_IN_CALL
   * @param text  - optional free-text custom status
   */
  setStatus(status?: string, text = ''): void {
    this._local.status = status || STATUS_ONLINE
    this._local.statusText = text || ''
    this._broadcast()
  }

  /** Stop presence: broadcast leave, clear timers. */
  leave(): void {
    this._stopped = true
    if (this._heartbeatTimer) clearInterval(this._heartbeatTimer)
    if (this._gcTimer) clearInterval(this._gcTimer)
    this._broadcastLeave()
    this._fabric.removeEventListener('message', this._onFabricMessage)
    this._fabric.removeEventListener('state', this._onFabricState)
  }

  /** Current roster snapshot (excludes self). Array of peer identity objects. */
  get roster(): RosterPeer[] {
    return [...this._roster.values()]
  }

  /** Full roster including the local user. */
  get fullRoster(): Array<(LocalPresence | RosterPeer) & { isSelf?: boolean }> {
    return [{ ...this._local, isSelf: true }, ...this.roster]
  }

  // ─── Internal ───────────────────────────────────────────────────────────────

  private _broadcast(): void {
    if (this._stopped) return
    this._local.ts = Date.now()
    this._sendPresenceFrame({ ...this._local, type: 'join' })
  }

  private _broadcastLeave(): void {
    this._sendPresenceFrame({ ...this._local, type: 'leave' })
  }

  private _sendPresenceFrame(payload: LocalPresence & { type: 'join' | 'leave' }): void {
    const frame = JSON.stringify({ channel: PRESENCE_CHANNEL, payload })
    this._fabric.send(frame)
  }

  private _handleMessage(ev: CustomEvent<{ from: string, data: string | ArrayBuffer }>): void {
    const { from, data } = ev.detail
    let text: string
    try {
      text = typeof data === 'string' ? data : new TextDecoder().decode(data)
    } catch { return }
    let frame: PresenceFrame
    try { frame = JSON.parse(text) } catch { return }
    if (frame.channel !== PRESENCE_CHANNEL) return
    const p = frame.payload
    if (!p || !p.accountId || p.accountId === this._local.accountId) return

    if (p.type === 'leave') {
      this._roster.delete(p.accountId)
    } else {
      this._roster.set(p.accountId, {
        accountId: p.accountId,
        displayName: p.displayName || 'Unknown',
        color: p.color || colorFromString(p.accountId),
        online: true,
        status: p.status || STATUS_ONLINE,          // OFFICE-62
        statusText: p.statusText || '',              // OFFICE-62
        isGuest: p.isGuest ?? false,
        ts: Date.now(),
        peerId: from,
      })
    }
    this._emitRoster()
  }

  private _handleState(ev: CustomEvent<{ state: string }>): void {
    // Re-announce ourselves whenever a new peer connects.
    const { state } = ev.detail
    if (state === 'connected' || state === 'relay') {
      this._broadcast()
    }
  }

  /** Remove peers that haven't sent a heartbeat within TIMEOUT_MS. */
  private _gc(): void {
    const now = Date.now()
    let changed = false
    for (const [id, peer] of this._roster) {
      if (now - peer.ts > TIMEOUT_MS) {
        this._roster.delete(id)
        changed = true
      }
    }
    if (changed) this._emitRoster()
  }

  private _emitRoster(): void {
    this.dispatchEvent(new CustomEvent('roster', { detail: this.fullRoster }))
  }
}

// ─── React hook ─────────────────────────────────────────────────────────────

import { useEffect, useRef, useState } from 'react'

export interface UsePresenceOptions {
  fabric: FabricClient | null
  localIdentity?: LocalIdentity | null
}

export interface UsePresenceResult {
  roster: Array<(LocalPresence | RosterPeer) & { isSelf?: boolean }>
  manager: PresenceManager | null
}

/**
 * usePresence — React hook that manages a PresenceManager lifecycle.
 *
 * Returns the full roster (including self with isSelf=true) while the fabric
 * is live; returns [] when fabric is null (editor opened without collab).
 * OFFICE-62: also returns manager so callers can call manager.setStatus(status, text).
 */
export function usePresence({ fabric, localIdentity = null }: UsePresenceOptions): UsePresenceResult {
  const [roster, setRoster] = useState<Array<(LocalPresence | RosterPeer) & { isSelf?: boolean }>>([])
  const pmRef = useRef<PresenceManager | null>(null)

  useEffect(() => {
    if (!fabric) {
      setRoster([])
      return
    }

    const pm = new PresenceManager({ fabric, localIdentity })
    pmRef.current = pm

    const onRoster = (ev: Event) => setRoster((ev as CustomEvent).detail)
    pm.addEventListener('roster', onRoster)
    pm.join()

    return () => {
      pm.removeEventListener('roster', onRoster)
      pm.leave()
      pmRef.current = null
    }
  }, [fabric]) // eslint-disable-line react-hooks/exhaustive-deps

  return { roster, manager: pmRef.current }
}
