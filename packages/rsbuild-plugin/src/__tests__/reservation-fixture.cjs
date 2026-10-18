const { mkdirSync, writeFileSync } = require('node:fs')
const { dirname, join } = require('node:path')

function put(root, path, code) {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, code)
}

function prepare(root, watch = false) {
  put(root, 'devup.json', '{}')
  put(
    root,
    'src/entry.js',
    'import {css} from "@devup-ui/react"; console.log(css({padding:"19px"}))',
  )
  put(
    root,
    'src/inventory.js',
    'export * from "../app/A.js"; export * from "../app/B.js"',
  )
  put(
    root,
    'src/unused.js',
    'import { globalCss } from "@devup-ui/react"; globalCss({".unused":{color:"chartreuse"}})',
  )
  put(
    root,
    'src/invalid.js',
    'import { css } from "@devup-ui/react"; export const broken = css({color: process.env.BAD})',
  )
  put(root, 'src/constants.js', 'export const tone = "red"')
  put(
    root,
    'node_modules/included-ui/package.json',
    '{"name":"included-ui","main":"dist/index.js"}',
  )
  put(
    root,
    'node_modules/included-ui/dist/index.js',
    'import {css} from "@devup-ui/react"; export const included = css({margin:"17px"})',
  )
  put(
    root,
    'app/Dynamic.jsx',
    'import {Box} from "@devup-ui/react"; export const dynamic = (color) => <Box color={color} styleOrder={0}/>',
  )
  if (!watch) addPair(root)
}

function addPair(root, order = 'AB') {
  for (const name of order) {
    put(
      root,
      `app/${name}.css.ts`,
      `import { createVar, createContainer, layer } from '@vanilla-extract/css';
export const variable = createVar(); export const container = createContainer(); export const layerName = layer();`,
    )
    put(
      root,
      `app/${name}.js`,
      `import { css, keyframes } from '@devup-ui/react';
import { tone } from '../src/constants.js';
import {variable, container, layerName} from './${name}.css.ts';
import {included} from 'included-ui';
import {dynamic} from './Dynamic.jsx';
export const animation = keyframes({from:{opacity:${name === 'A' ? '0' : '0.25'}},to:{opacity:1}});
export const value = css({color:${name === 'A' ? 'tone' : '"blue"'}, animationName:animation});
console.log(${JSON.stringify(name)}, value, animation, variable, container, layerName, included, dynamic);`,
    )
  }
}

module.exports = { put, prepare, addPair }
