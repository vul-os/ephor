// emitter.ts — minimal event emitter shared across call modules.
//
// Provides on/off/emit semantics (EventTarget-ish but lighter weight).
// Used by rtc.ts (the Call class extends it) and fabricSignaling.ts
// (the BroadcastChannel stub session exposes the same surface).

/** Map of event name -> the tuple of argument types emitted for it. */
export type EventMap = Record<string, unknown[]>

/** A loosely-typed event map for callers that don't declare one. */
export type AnyEventMap = Record<string, unknown[]>

export class Emitter<Events extends EventMap = AnyEventMap> {
  private _h: { [K in keyof Events]?: Array<(...args: Events[K]) => void> }

  constructor() { this._h = {} }

  on<K extends keyof Events>(ev: K, cb: (...args: Events[K]) => void): () => void {
    (this._h[ev] = this._h[ev] || []).push(cb)
    return () => this.off(ev, cb)
  }

  off<K extends keyof Events>(ev: K, cb: (...args: Events[K]) => void): void {
    this._h[ev] = (this._h[ev] || []).filter(f => f !== cb)
  }

  emit<K extends keyof Events>(ev: K, ...a: Events[K]): void {
    (this._h[ev] || []).forEach(f => { try { f(...a) } catch (e) { console.error(e) } })
  }
}
