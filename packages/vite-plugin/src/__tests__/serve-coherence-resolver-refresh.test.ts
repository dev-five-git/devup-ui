import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import { fixture } from '../../../plugin-utils/src/__tests__/reservation-fixture'
import { createViteReservation } from '../reservation'

it('refreshes live alias resolution before dirty reservation without refreshing clean preparation', async () => {
  // Given
  using project = fixture()
  const paths = (color: string) =>
    JSON.stringify({
      compilerOptions: {
        baseUrl: '.',
        paths: { '@tone': [`./src/${color}.js`] },
      },
    })
  const source =
    "import {Box} from '@devup-ui/react'; import {tone} from '@tone'; import './red.js'; import './blue.js'; export const A=()=> <Box color={tone}/>;"
  project.write('src/A.tsx', source)
  project.write('src/red.js', "export const tone='red'")
  project.write('src/blue.js', "export const tone='blue'")
  project.write('tsconfig.json', paths('red'))
  const slots = createViteReservation({
    package: '@devup-ui/react',
    include: [],
    importAliases: {},
    singleCss: true,
    cssDir: join(project.root, 'df/css'),
    distDir: join(project.root, 'df'),
  })
  slots.configure({ root: project.root })
  using resolver = spyOn(wasm, 'setModuleResolver')
  await slots.reserve()
  const id = join(project.root, 'src/A.tsx').replaceAll('\\', '/')
  using before = wasm.codeExtract(
    id,
    source,
    '@devup-ui/react',
    '../df/css',
    true,
    true,
    false,
    {},
  )
  expect(before.dependencies).toContain(
    join(project.root, 'src/red.js').replaceAll('\\', '/'),
  )
  await slots.reserve()
  expect(resolver).toHaveBeenCalledTimes(1)
  project.write('tsconfig.json', paths('blue'))
  slots.invalidate(join(project.root, 'tsconfig.json'))
  // When
  await slots.reserve()
  using after = wasm.codeExtract(
    id,
    source,
    '@devup-ui/react',
    '../df/css',
    true,
    true,
    false,
    {},
  )
  // Then
  expect(after.dependencies).toContain(
    join(project.root, 'src/blue.js').replaceAll('\\', '/'),
  )
  expect(after.code).not.toBe(before.code)
  expect(resolver).toHaveBeenCalledTimes(2)
})
