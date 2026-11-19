// Compiled output imports runtime unit conversions from `@devup-ui/react/scale`.
// Fixture projects built in the system temp directory have no packages
// installed, so the package's `scale` subpath is installed where any of them
// resolves it, as an app has it installed beside Devup UI.
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const directory = join(tmpdir(), 'node_modules', '@devup-ui', 'react')
const source = new Bun.Transpiler({ loader: 'ts' }).transformSync(
  readFileSync(join(import.meta.dir, 'packages/react/src/scale.ts'), 'utf8'),
)
mkdirSync(directory, { recursive: true })
writeFileSync(
  join(directory, 'package.json'),
  JSON.stringify({
    name: '@devup-ui/react',
    sideEffects: false,
    exports: {
      './scale': { import: './scale.mjs', require: './scale.cjs' },
    },
  }),
)
writeFileSync(join(directory, 'scale.mjs'), source)
writeFileSync(
  join(directory, 'scale.cjs'),
  `${source.replaceAll('export function', 'function')}\nmodule.exports = { __devupScale, __devupNativeScale }\n`,
)
