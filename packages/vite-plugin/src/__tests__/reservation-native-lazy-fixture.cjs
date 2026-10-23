const { existsSync, symlinkSync } = require('node:fs')
const { dirname, join } = require('node:path')

/** @param {string} root @param {(path:string,code:string)=>void} write */
function lazyFixture(root, write) {
  if (!existsSync(join(root, 'node_modules/react')))
    symlinkSync(
      dirname(
        require.resolve('react/package.json', {
          paths: [join(__dirname, '../../../react')],
        }),
      ),
      join(root, 'node_modules/react'),
      'junction',
    )
  write(
    'index.html',
    '<script type="module" src="/root.js"></script><script type="module" src="/aux.js"></script>',
  )
  write('root.js', "import './physical/A.tsx'; import('./physical/B.tsx')")
  write('aux.js', "import './physical/B.tsx'; import './shared/P.tsx'")
  write(
    'shared/S.tsx',
    "import {styled} from '@devup-ui/react'; export const Shared=styled('div',{color:'orchid',p:7})",
  )
  write(
    'shared/P.tsx',
    "import {styled} from '@devup-ui/react'; export const Private=styled('div',{borderColor:'teal',p:9}); console.log(Private)",
  )
  write(
    'physical/B.tsx',
    "import {Box} from '@devup-ui/react'; import {Shared} from '../shared/S'; export const B=(value:string)=><><Shared/><Box opacity={.75} w={value}/></>; console.log(B)",
  )
  write('src/unused-lazy.js', "import('../E.js')")
  write('E.js', "import './shared/P.tsx'")
  write('shared/tone.js', "export const tone='silver'")
  write(
    'physical/A.tsx',
    "import {Box} from '@devup-ui/react'; import {Shared} from '../shared/S'; import {Private} from '../shared/P'; import {tone} from '../shared/tone'; export const A=(value:string)=><><Shared/><Private/><Box opacity={.25} color={tone} w={value}/></>; console.log(A)",
  )
  return [
    '/root.js',
    '/aux.js',
    '/physical/A.tsx',
    '/physical/B.tsx',
    '/shared/S.tsx',
    '/shared/P.tsx',
  ]
}
module.exports = { lazyFixture }
