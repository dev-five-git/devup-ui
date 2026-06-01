import { register } from './plugin'

export { plugin } from './plugin'

// Top-level await: Bun's preload mechanism suspends until this module finishes
// evaluating, so awaiting registration guarantees the plugin's onLoad hook is
// installed before any test/source file is loaded. This is the ESM entry
// point; top-level await is valid here and survives the esm build.
await register()
