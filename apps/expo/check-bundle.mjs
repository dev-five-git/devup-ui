// Checks the bundles `expo export` wrote to dist/ for iOS and Android: Devup
// UI compiled away at build time, so no bundle may hold the runtime
// placeholders of @devup-ui/react or any module of that package but its React
// Native runtime (`@devup-ui/react/native`), and the styles of src/App.tsx
// must be there as React Native style objects. src/App.tsx reads the theme,
// so it uses the runtime; src/Badge.tsx is styled the same in every theme, so
// it must not.
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

const PLACEHOLDER = 'Cannot run on the runtime'
/** Names of the runtime that only code using it holds, as minified code keeps them */
const RUNTIME_NAMES = /\b(?:defineThemes|useThemeIndex)\b/g
/** Modules of @devup-ui/react, by the path Metro resolves */
const REACT_PACKAGE = /(^|[\\/])(@devup-ui[\\/]react|packages[\\/]react)[\\/]/
/** The React Native runtime among them */
const NATIVE_RUNTIME =
  /(^|[\\/])(@devup-ui[\\/]react|packages[\\/]react)[\\/]dist[\\/]native[\\/]/
const root = join(import.meta.dirname, 'dist', '_expo', 'static', 'js')
const failures = []

const BASE64 =
  'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'

/** Each generated line of `mappings`, as its segments' [column, source index] */
function decodeMappings(mappings) {
  let source = 0
  return mappings.split(';').map((line) => {
    let column = 0
    const segments = []
    for (const segment of line.split(',')) {
      if (!segment) continue
      const fields = []
      let value = 0
      let shift = 0
      for (const character of segment) {
        const digit = BASE64.indexOf(character)
        value += (digit & 31) << shift
        if (digit & 32) {
          shift += 5
        } else {
          fields.push(value & 1 ? -(value >>> 1) : value >>> 1)
          value = 0
          shift = 0
        }
      }
      column += fields[0]
      if (fields.length > 1) {
        source += fields[1]
        segments.push([column, source])
      }
    }
    return segments
  })
}

/** A function telling the source a generated line and column come from */
function sourceLocator(map) {
  if (map.sections) {
    const sections = map.sections.map(({ offset, map }) => ({
      offset,
      locate: sourceLocator(map),
    }))
    return (line, column) => {
      const section = sections.findLast(
        ({ offset }) =>
          offset.line < line ||
          (offset.line === line && offset.column <= column),
      )
      if (!section) return null
      const { offset } = section
      return section.locate(
        line - offset.line,
        line === offset.line ? column - offset.column : column,
      )
    }
  }
  const lines = decodeMappings(map.mappings)
  return (line, column) => {
    const segment = lines[line]?.findLast(([start]) => start <= column)
    return segment ? map.sources[segment[1]] : null
  }
}

/** The sources of a source map, an indexed one (Metro's) included */
function mapSources(map) {
  return map.sections
    ? map.sections.flatMap((section) => mapSources(section.map))
    : map.sources
}

/** The sources of the code that names the runtime */
function runtimeUsers(code, locate) {
  const users = new Set()
  code.split('\n').forEach((line, index) => {
    for (const match of line.matchAll(RUNTIME_NAMES)) {
      users.add(locate(index, match.index))
    }
  })
  return [...users].filter(
    (source) => source !== null && !NATIVE_RUNTIME.test(source),
  )
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
    const map = JSON.parse(
      readFileSync(join(directory, `${file}.map`), 'utf-8'),
    )
    const sources = mapSources(map)
    for (const source of sources) {
      if (REACT_PACKAGE.test(source) && !NATIVE_RUNTIME.test(source)) {
        failures.push(`${platform}/${file}: bundles ${source}`)
      }
    }
    if (!sources.some((source) => NATIVE_RUNTIME.test(source))) {
      failures.push(`${platform}/${file}: lacks @devup-ui/react/native`)
    }
    const users = runtimeUsers(code, sourceLocator(map))
    if (!users.some((source) => /src[\\/]App\.tsx$/.test(source))) {
      failures.push(`${platform}/${file}: src/App.tsx does not use the runtime`)
    }
    for (const source of users.filter((source) =>
      /src[\\/]Badge\.tsx$/.test(source),
    )) {
      failures.push(`${platform}/${file}: ${source} uses the runtime`)
    }
    console.info(
      `${platform}/${file}: ${sources.length} modules, runtime used by ${users.join(', ')}`,
    )
  }
}

if (failures.length > 0) {
  console.error(failures.join('\n'))
  process.exit(1)
}
