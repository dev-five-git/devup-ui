import { keyframes } from '@devup-ui/react'

import { a } from './deep/a'

// A transitive import chain (a -> b -> c -> d -> e) grows the module graph past
// the window in which the old, un-awaited Bun.plugin() registration used to
// win the race. With more modules to load, an unresolved async setup() reliably
// loses and the compile-only call below throws.
export const transitive = a

// Compile-only API exercised at the top level of the module. When the plugin's
// onLoad hook is installed before this module is loaded (i.e. the entry point
// awaited Bun.plugin()), this is rewritten to a static string at load time.
// If the plugin lost the preload race, this would instead reach the
// @devup-ui/react runtime stub and throw "Cannot run on the runtime".
export const fade = keyframes({
  from: { opacity: 0 },
  to: { opacity: 1 },
})
