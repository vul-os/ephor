/// <reference types="vite/client" />

// Augments Vite's own ImportMetaEnv (which otherwise types every env var as
// `any` via an index signature) with the specific build-time vars this
// console actually reads (src/lib/api.ts's createClient()). This is what
// removes the no-unsafe-* findings at every import.meta.env.VITE_* call site
// instead of casting each one individually.
interface ImportMetaEnv {
  /** '0' selects RealAdminClient; anything else (including unset) is mock — see README. */
  readonly VITE_MOCK?: string;
  /** Base URL for the real admin API when VITE_MOCK !== '0'. */
  readonly VITE_API_BASE?: string;
  /** Bearer token fallback when localStorage has none yet (dev convenience only). */
  readonly VITE_ADMIN_TOKEN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
