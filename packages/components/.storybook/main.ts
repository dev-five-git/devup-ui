import { DevupUI } from '@devup-ui/vite-plugin'
import { type StorybookConfig } from '@storybook/react-vite'
import { dirname, join } from 'path'
import { mergeConfig } from 'vite'

function getAbsolutePath(value: string) {
  return dirname(require.resolve(join(value, 'package.json')))
}

const config: StorybookConfig = {
  stories: ['../src/**/*.mdx', '../src/**/*.stories.@(js|jsx|mjs|ts|tsx)'],

  addons: [
    getAbsolutePath('@storybook/addon-onboarding'),
    getAbsolutePath('@storybook/addon-links'),
    getAbsolutePath('@storybook/addon-essentials'),
    getAbsolutePath('@chromatic-com/storybook'),
    '@chromatic-com/storybook'
  ],

  framework: {
    name: '@storybook/react-vite',
    options: {},
  },

  viteFinal(config) {
    return mergeConfig(config, {
      plugins: [DevupUI()],
    })
  },

  typescript: {
    reactDocgen: 'react-docgen-typescript'
  }
}
export default config
