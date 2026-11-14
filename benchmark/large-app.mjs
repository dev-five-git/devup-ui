// Writes the generated source of the large-app benchmark fixtures.
//
// The other fixtures are one page, which hides what grows with an app: import
// resolution, barrels, constants read across modules, and the work done once per
// module. This app has ROUTES pages, each rendering COMPONENTS_PER_ROUTE of the
// COMPONENTS components through one barrel, all reading shared tokens.
//
//   node ../large-app.mjs   (from a fixture directory; files already up to date are left alone)
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

const ROUTES = 30
const COMPONENTS = 300
const COMPONENTS_PER_ROUTE = 10
const TONES = 20

const pad = (value, width) => String(value).padStart(width, '0')
const component = (index) => `C${pad(index, 3)}`

function write(path, content) {
  const target = join(process.cwd(), 'src', path)
  if (existsSync(target) && readFileSync(target, 'utf8') === content) return
  mkdirSync(dirname(target), { recursive: true })
  writeFileSync(target, content)
}

const tones = Array.from(
  { length: TONES },
  (_, index) =>
    `  t${index}: '#${pad(((index * 0x0b3d17) % 0xffffff).toString(16), 6)}',`,
)
write(
  'tokens.ts',
  `export const tones = {\n${tones.join('\n')}\n} as const\n\nexport const space = { s: 4, m: 8, l: 16 } as const\n`,
)

for (let index = 0; index < COMPONENTS; index++) {
  const name = component(index)
  write(
    `components/${name}.tsx`,
    `import { Box, css, Flex, styled, Text } from '@devup-ui/react'

import { space, tones } from '../tokens'

const Frame = styled('section', {
  borderRadius: ${4 + (index % 8)},
  p: space.m,
})

export function ${name}({ active }: { active?: boolean }) {
  return (
    <Frame>
      <Flex className={css({ gap: ${2 + (index % 12)} })} flexDir={['column', 'row']}>
        <Box
          _hover={{ opacity: ${((index % 9) + 1) / 10} }}
          bg={tones.t${index % TONES}}
          color={active ? '$text' : 'gray'}
          p={space.${['s', 'm', 'l'][index % 3]}}
        >
          <Text fontSize={[12, ${12 + (index % 10)}]}>item ${index}</Text>
        </Box>
      </Flex>
    </Frame>
  )
}
`,
  )
}

write(
  'components/index.ts',
  Array.from(
    { length: COMPONENTS },
    (_, index) => `export { ${component(index)} } from './${component(index)}'`,
  ).join('\n') + '\n',
)

const routes = []
for (let route = 0; route < ROUTES; route++) {
  const names = Array.from({ length: COMPONENTS_PER_ROUTE }, (_, offset) =>
    component((route * COMPONENTS_PER_ROUTE + offset) % COMPONENTS),
  )
  const path = `r${pad(route, 2)}`
  routes.push(path)
  write(
    `app/${path}/page.tsx`,
    `import { ${names.join(', ')} } from '@/components'

export default function Page() {
  return (
    <main>
${names.map((name, offset) => `      <${name} active={${offset % 2 === 0}} />`).join('\n')}
    </main>
  )
}
`,
  )
}

write(
  'app/page.tsx',
  `import { Box } from '@devup-ui/react'

export default function Page() {
  return (
    <Box as="nav" p={4}>
${routes.map((path) => `      <a href="/${path}">${path}</a>`).join('\n')}
    </Box>
  )
}
`,
)
