import { plugin } from 'bun'

import { DevupUI, type DevupUIBunPluginOptions } from './devup-plugin'

// Await async setup before runtime imports, including Bun preload evaluation.
function register(options: DevupUIBunPluginOptions = {}) {
  return plugin(DevupUI(options))
}

export type { DevupUIBunPluginOptions } from './devup-plugin'
export { DevupUI, plugin, register }
