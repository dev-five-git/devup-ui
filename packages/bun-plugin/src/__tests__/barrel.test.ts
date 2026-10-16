import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { createModuleResolver } from '@devup-ui/plugin-utils'
import {
  codeExtract,
  hasDevupUI,
  registerTheme,
  setModuleResolver,
} from '@devup-ui/wasm'
import { afterAll, beforeAll, describe, expect, it } from 'bun:test'

const files: Record<string, string> = {
  'tsconfig.json': JSON.stringify({
    compilerOptions: { baseUrl: '.', paths: { '@ui/*': ['src/ui/*'] } },
  }),
  'src/ui/index.ts': `export { Box, css } from '@devup-ui/react'\nexport * from './more'\n`,
  'src/ui/more.ts': `export { Text } from '@devup-ui/react'\n`,
  'src/ui/broken.ts': `export * from './nowhere'\nexport { Grid } from '@devup-ui/react'\n`,
  'node_modules/@acme/ui/package.json': JSON.stringify({
    name: '@acme/ui',
    exports: { '.': { import: './dist/index.js' } },
  }),
  'node_modules/@acme/ui/dist/index.js': `export { Flex } from '@devup-ui/react'\n`,
  'src/relative.tsx': `import { Box, Text } from './ui'\nexport const a = <Box bg="red"><Text color="blue" /></Box>\n`,
  'src/paths.tsx': `import { Box } from '@ui/index'\nexport const a = <Box bg="red" />\n`,
  'src/package.tsx': `import { Flex } from '@acme/ui'\nexport const a = <Flex gap={2} />\n`,
  'src/namespace.tsx': `import * as UI from './ui'\nconst { css } = UI\nexport const a = <UI.Box bg="red" className={css({ m: 1 })} />\n`,
  'src/unfollowable.tsx': `import { Grid, Missing } from './ui/broken'\nexport const a = <Missing />\n`,
  'src/plain.tsx': `import { helper } from './ui/more'\nexport const a = helper\n`,
}

let root = ''

function read(name: string) {
  return {
    filename: join(root, name).replaceAll('\\', '/'),
    code: files[name],
  }
}

function extract(name: string) {
  const { filename, code } = read(name)
  return codeExtract(
    filename,
    code,
    '@devup-ui/react',
    '@devup-ui/react',
    true,
    false,
    false,
    {},
  )
}

beforeAll(() => {
  root = mkdtempSync(join(tmpdir(), 'devup-barrel-'))
  for (const [name, code] of Object.entries(files)) {
    const file = join(root, name)
    mkdirSync(dirname(file), { recursive: true })
    writeFileSync(file, code)
  }
  registerTheme({})
  setModuleResolver(
    createModuleResolver({
      cwd: root,
      toId: (path) => path.replaceAll('\\', '/'),
    }),
  )
})

afterAll(() => {
  setModuleResolver(createModuleResolver())
  rmSync(root, { recursive: true, force: true })
})

describe('Devup UI re-exported by a project module', () => {
  it.each(['relative', 'paths', 'package', 'namespace'])(
    'is found in %s.tsx by the gate and compiled',
    (name) => {
      const { filename, code } = read(`src/${name}.tsx`)
      expect(hasDevupUI(filename, code, '@devup-ui/react')).toBe(true)
      const output = extract(`src/${name}.tsx`)
      expect(output.code).toContain('<div className=')
      expect(output.code).not.toContain('<Box')
      expect(output.dependencies.length).toBeGreaterThan(0)
    },
  )

  it('leaves a file that never reaches Devup UI to the gate', () => {
    const { filename, code } = read('src/plain.tsx')
    expect(hasDevupUI(filename, code, '@devup-ui/react')).toBe(false)
  })

  it('reports what it cannot follow where it is used', () => {
    const { filename, code } = read('src/unfollowable.tsx')
    expect(hasDevupUI(filename, code, '@devup-ui/react')).toBe(true)
    expect(() => extract('src/unfollowable.tsx')).toThrow(
      /unfollowable\.tsx:2:19: `Missing` cannot use `\.\/ui\/broken` at build time/,
    )
  })
})
