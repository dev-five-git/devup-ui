import { DevupUI } from '@devup-ui/vite-plugin'
import type { StorybookConfig } from '@storybook/react-vite'
import { mergeConfig } from 'vite'

const config: StorybookConfig = {
  stories: ['../src/**/*.mdx', '../src/**/*.stories.@(js|jsx|mjs|ts|tsx)'],
  addons: ['@storybook/addon-docs', '@storybook/addon-onboarding'],
  framework: {
    name: '@storybook/react-vite',
    options: {},
  },

  viteFinal(config) {
    return mergeConfig(config, {
      plugins: [DevupUI()],
    })
  },
}
export default config
