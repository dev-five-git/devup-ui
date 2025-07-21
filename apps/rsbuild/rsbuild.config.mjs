import { defineConfig } from '@rsbuild/core';
import { pluginReact } from '@rsbuild/plugin-react';
import { DevupUIRsbuildPlugin } from '@devup-ui/rsbuild-plugin';

export default defineConfig({
  plugins: [pluginReact(), DevupUIRsbuildPlugin()],
});
