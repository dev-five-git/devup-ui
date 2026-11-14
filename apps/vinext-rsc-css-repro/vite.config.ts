import { DevupUI } from '@devup-ui/vite-plugin'
import vinext from 'vinext'
import { defineConfig } from 'vite'

const engine = process.env.DEVUP_ENGINE
switch (engine) {
  case undefined:
  case 'native':
  case 'wasm':
    break
  default:
    throw new TypeError(`Invalid DEVUP_ENGINE: ${JSON.stringify(engine)}`)
}

export default defineConfig({
  plugins: [
    DevupUI({ engine, singleCss: process.env.DEVUP_SINGLE_CSS === '1' }),
    vinext(),
  ],
})
