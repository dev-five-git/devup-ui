import { DevupUI } from '@devup-ui/vite-plugin'
import vinext from 'vinext'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [DevupUI(), vinext()],
})
