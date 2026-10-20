import { existsSync, readFileSync } from 'node:fs'
import { cp, mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const repository = fileURLToPath(new URL('../../', import.meta.url))
const templates = fileURLToPath(new URL('./fixtures/', import.meta.url))

function installedVersion(name, context) {
  const require = createRequire(join(repository, context, 'package.json'))
  let directory = dirname(require.resolve(name))
  for (;;) {
    const manifestPath = join(directory, 'package.json')
    if (existsSync(manifestPath)) {
      const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))
      if (typeof manifest.version === 'string') return manifest.version
    }
    const parent = dirname(directory)
    if (parent === directory)
      throw new Error(`Cannot locate installed manifest for ${name}`)
    directory = parent
  }
}

export async function prepareConsumer(name, artifacts, run) {
  const directory = await mkdtemp(join(artifacts.root, `${name}-`))
  await mkdir(join(directory, 'src'), { recursive: true })
  for (const filename of ['Fixture.jsx', 'styles.mjs', 'main.jsx'])
    await cp(join(templates, filename), join(directory, 'src', filename))
  for (const filename of ['devup.json', 'base-theme.json', 'index.html'])
    await cp(join(templates, filename), join(directory, filename))
  const isNext = name.startsWith('next-')
  const plugin = isNext ? 'next-plugin' : `${name}-plugin`
  const dependencies = {
    react: installedVersion('react', 'apps/vite'),
    'react-dom': installedVersion('react-dom', 'apps/vite'),
    '@devup-ui/react': artifacts.tarballs.react,
    [`@devup-ui/${plugin}`]: artifacts.tarballs[plugin],
  }
  if (name === 'vite') {
    dependencies.vite = installedVersion('vite', 'apps/vite')
    dependencies['@vitejs/plugin-react'] = installedVersion(
      '@vitejs/plugin-react',
      'apps/vite',
    )
  }
  if (name === 'rsbuild') {
    dependencies['@rsbuild/core'] = installedVersion(
      '@rsbuild/core',
      'apps/rsbuild',
    )
    dependencies['@rsbuild/plugin-react'] = installedVersion(
      '@rsbuild/plugin-react',
      'apps/rsbuild',
    )
  }
  if (isNext || name === 'webpack')
    dependencies.next = installedVersion('next', 'apps/next')
  if (isNext) {
    await mkdir(join(directory, 'app'))
    for (const filename of ['page.jsx', 'layout.jsx'])
      await cp(join(templates, filename), join(directory, 'app', filename))
  }
  if (name === 'webpack') {
    dependencies['@typescript/typescript6'] = installedVersion(
      '@typescript/typescript6',
      '.',
    )
    dependencies.typescript = installedVersion('typescript', '.')
    for (const filename of ['build-webpack.cjs', 'jsx-loader.cjs'])
      await cp(join(templates, filename), join(directory, filename))
  }
  await writeFile(
    join(directory, 'package.json'),
    JSON.stringify(
      {
        name: `release-${name}`,
        private: true,
        type: 'module',
        dependencies,
        overrides: Object.fromEntries(
          Object.entries(artifacts.tarballs).map(([item, path]) => [
            `@devup-ui/${item}`,
            path,
          ]),
        ),
      },
      null,
      2,
    ),
  )
  const config = isNext
    ? 'next.config.mjs'
    : name === 'webpack'
      ? 'webpack.config.cjs'
      : `${name}.config.mjs`
  await cp(join(templates, config), join(directory, config))
  await run(process.env.BUN_BINARY ?? 'bun', ['install'], { cwd: directory })
  return directory
}
