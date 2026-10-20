import { relative } from 'node:path'

import { expect, it } from 'bun:test'

import { __setOxcParserForTest } from '../import-graph'
import { collectReservationInputs } from '../reservation-discovery'
import { fixture } from './reservation-fixture'

const red =
  'import {css} from "@devup-ui/react"; export const cls=css({color:"red"});'

it('ignores ordinary quoted import payloads in the existing fallback scanner seam', () => {
  // Given
  using fs = fixture()
  fs.write(
    'src/entry.js',
    `${red}\nexport const snippet = "import '../out/stale.js'";`,
  )
  fs.write('out/stale.js', red.replace('red', 'chartreuse'))
  __setOxcParserForTest(false)
  try {
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
    console.info(JSON.stringify({ audit: 'quoted-inventory', paths }))
    // Then
    expect(paths).toEqual(['src/entry.js'])
  } finally {
    __setOxcParserForTest(undefined)
  }
})
