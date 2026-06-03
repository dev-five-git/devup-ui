import { register } from './plugin'

export { plugin } from './plugin'

// CJS entry point. `bun build --format cjs` cannot emit top-level await, so we
// register eagerly and surface any rejection instead. Bun's preload always
// loads the ESM build (dist/index.mjs / the TS source), which uses real
// top-level await to close the race window; this CJS path is a best-effort
// fallback for require()-based consumers.
register().catch((error: unknown) => {
  throw error
})
