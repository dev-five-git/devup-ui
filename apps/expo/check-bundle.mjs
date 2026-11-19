// Checks the bundles `expo export` wrote to dist/ for iOS and Android: Devup
// UI compiled away at build time, so no bundle may hold the runtime
// placeholders of @devup-ui/react or any module of that package, and the
// styles of src/App.tsx must be there as React Native style objects.
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

const PLACEHOLDER = 'Cannot run on the runtime'
const root = join(import.meta.dirname, 'dist', '_expo', 'static', 'js')
const failures = []

/** The sources of a source map, an indexed one (Metro's) included */
function mapSources(map) {
  return map.sections
    ? map.sections.flatMap((section) => mapSources(section.map))
    : map.sources
}

for (const platform of ['ios', 'android']) {
  const directory = join(root, platform)
  const bundles = readdirSync(directory).filter((file) => file.endsWith('.js'))
  if (bundles.length === 0) failures.push(`${platform}: no JavaScript bundle`)
  for (const file of bundles) {
    const code = readFileSync(join(directory, file), 'utf-8')
    if (code.includes(PLACEHOLDER)) {
      failures.push(`${platform}/${file}: holds "${PLACEHOLDER}"`)
    }
    for (const style of ['backgroundColor:"#5a44ff"', 'StyleSheet']) {
      if (!code.replaceAll(' ', '').includes(style)) {
        failures.push(`${platform}/${file}: lacks ${style}`)
      }
    }
    const sources = mapSources(
      JSON.parse(readFileSync(join(directory, `${file}.map`), 'utf-8')),
    )
    for (const source of sources) {
      if (
        /(^|[\\/])(@devup-ui[\\/]react|packages[\\/]react)[\\/]/.test(source)
      ) {
        failures.push(`${platform}/${file}: bundles ${source}`)
      }
    }
    console.info(`${platform}/${file}: ${sources.length} modules, checked`)
  }
}

if (failures.length > 0) {
  console.error(failures.join('\n'))
  process.exit(1)
}
