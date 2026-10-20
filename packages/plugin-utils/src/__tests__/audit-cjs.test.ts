import { relative } from 'node:path'

import { expect, it } from 'bun:test'

import { collectReservationInputs } from '../reservation-discovery'
import { fixture } from './reservation-fixture'

const red =
  'import {css} from "@devup-ui/react"; export const cls=css({color:"red"});'

it('includes physical literal require leaves of an implicit allowed package', () => {
  // Given
  using fs = fixture()
  fs.write(
    'src/entry.js',
    'import {A,B} from "@devup-ui/widgets"; console.log(A,B);',
  )
  fs.write(
    'node_modules/@devup-ui/widgets/package.json',
    '{"main":"dist/index.js"}',
  )
  fs.write(
    'node_modules/@devup-ui/widgets/dist/index.js',
    'exports.A=require("./A.js"); exports.B=require("./B.js");',
  )
  fs.write('node_modules/@devup-ui/widgets/dist/A.js', red)
  fs.write(
    'node_modules/@devup-ui/widgets/dist/B.js',
    red.replace('red', 'blue'),
  )
  // When
  const inventory = collectReservationInputs({
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  })
  const paths = inventory.inputs.map(({ physicalPath }) =>
    relative(fs.root, physicalPath).replaceAll('\\', '/'),
  )
  console.info(JSON.stringify({ audit: 'require-inventory', paths }))
  // Then
  expect(paths).toContain('node_modules/@devup-ui/widgets/dist/A.js')
  expect(paths).toContain('node_modules/@devup-ui/widgets/dist/B.js')
})
