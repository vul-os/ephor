// `virtual:brand-mark/<brand>` is served by the `brandMark` plugin in
// vite.config.ts: it reads the brand's `markPath` off disk and hands back the
// SVG source as a default-exported string. One module id per brand (rather than
// one shared id) so importing a brand module never silently yields a different
// brand's artwork.
declare module 'virtual:brand-mark/*' {
  const svg: string;
  export default svg;
}

// The `$brand` alias itself is resolved by vite.config.ts at build time and
// mapped to the default brand in tsconfig.app.json's `paths` for typechecking —
// no ambient declaration, so the editor still sees the real `Brand` type.
