module.exports = function deliveryFixture(put) {
  const singleCss = process.env.DEVUP_NEXT_SINGLE_CSS === '1'
  put(
    'devup.json',
    JSON.stringify({ theme: { colors: { default: { proof: '#123456' } } } }),
  )
  put(
    'next.config.js',
    `
    const { DevupUI } = require('@devup-ui/next-plugin');
    module.exports = DevupUI({
      experimental: { cpus: 1, webpackBuildWorker: true },
      transpilePackages: ['delivery-library'],
      webpack(config, context) {
        console.info('TOPOLOGY ' + JSON.stringify({ pid: process.pid, threadId: require('node:worker_threads').threadId, isServer: context.isServer, nextRuntime: context.nextRuntime || 'client' }));
        return config;
      }
    }, { singleCss: ${singleCss}, include: ['delivery-library'] });
  `,
  )
  put(
    'src/app/layout.js',
    `
    import { Box, globalCss } from '@devup-ui/react';
    globalCss({ body: { background: '#040', color: '#248' } });
    export default function Layout({ children }) {
      return <html><body><Box id="layout-proof" color="white">{children}</Box></body></html>;
    }
  `,
  )
  put(
    'src/app/page.js',
    `
    import { Box } from '@devup-ui/react';
    import { Child } from 'delivery-library';
    import Client from './client';
    export default function Page() {
      return <><Box id="server-proof" bg="maroon" p="13px">server-proof</Box><Client /><Child id="external-proof">external-proof</Child></>;
    }
  `,
  )
  put(
    'src/app/client.js',
    `
    'use client';
    import { Box } from '@devup-ui/react';
    export default function Client() { return <Box id="client-proof" color="cyan">client-proof</Box>; }
  `,
  )
  put('src/app/client/page.js', `export { default } from '../client';`)
  put(
    'src/pages/edge-page.js',
    `
    import { Box, css } from '@devup-ui/react';
    export const config = { runtime: 'experimental-edge' };
    const edgePage = css({ color: 'goldenrod', padding: '19px' });
    export default function EdgePage() { return <Box id="layout-proof" color="white"><div id="edge-page-proof" className={edgePage}>edge-page-proof</div></Box>; }
  `,
  )
  put(
    'node_modules/delivery-library/package.json',
    JSON.stringify({
      name: 'delivery-library',
      version: '1.0.0',
      type: 'module',
      main: './index.mjs',
    }),
  )
  put(
    'node_modules/delivery-library/index.mjs',
    `export { Child } from './child.mjs';`,
  )
  put(
    'node_modules/delivery-library/child.mjs',
    `
    import { styled } from '@devup-ui/react';
    export const Child = styled('strong', { color: 'rebeccapurple', marginLeft: '17px' });
  `,
  )
}
