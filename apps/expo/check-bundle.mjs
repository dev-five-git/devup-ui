// Checks the bundles `expo export` wrote to dist/ for iOS and Android: Devup
// UI compiled away at build time, so no bundle may hold the runtime
// placeholders of @devup-ui/react or any module of that package but its React
// Native runtime (`@devup-ui/react/native`), and the styles of src/App.tsx
// must be there as React Native style objects. src/App.tsx reads the theme,
// the breakpoint and the orientation, so it calls each of those hooks of the
// runtime; src/Badge.tsx is styled the same in every theme and at every
// width, so it must not use the runtime. Its buttons take press styles, which
// React Native's `Pressable` applies through a function for `style`, and its
// input a focus style, which the runtime's `Focused` applies.
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

const PLACEHOLDER = 'Cannot run on the runtime'
/** Names of the runtime that only code using it holds, as minified code keeps them */
const RUNTIME_NAMES =
  /\b(?:defineThemes|useThemeIndex|useBreakpoint|useLandscape|Focused)\b/g
/** What src/App.tsx must use from the runtime */
const APP_HOOKS = ['useThemeIndex', 'useBreakpoint', 'useLandscape', 'Focused']
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

/** Each name of the runtime, with the sources of the code that names it */
function runtimeUsers(code, locate) {
  const users = new Map()
  code.split('\n').forEach((line, index) => {
    for (const match of line.matchAll(RUNTIME_NAMES)) {
      const source = locate(index, match.index)
      if (source === null || NATIVE_RUNTIME.test(source)) continue
      const sources = users.get(match[0]) ?? new Set()
      sources.add(source)
      users.set(match[0], sources)
    }
  })
  return users
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
    for (const style of [
      'backgroundColor:"#5a44ff"',
      'backgroundColor:"#4330d9"',
      'StyleSheet',
      'style:({pressed:',
    ]) {
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
    for (const hook of APP_HOOKS) {
      const callers = [...(users.get(hook) ?? [])]
      if (!callers.some((source) => /src[\\/]App\.tsx$/.test(source))) {
        failures.push(`${platform}/${file}: src/App.tsx does not use ${hook}`)
      }
    }
    const all = [
      ...new Set([...users.values()].flatMap((sources) => [...sources])),
    ]
    for (const source of all.filter((source) =>
      /src[\\/]Badge\.tsx$/.test(source),
    )) {
      failures.push(`${platform}/${file}: ${source} uses the runtime`)
    }
    console.info(
      `${platform}/${file}: ${sources.length} modules, runtime used by ${all.join(', ')}`,
    )
  }
}

if (failures.length > 0) {
  console.error(failures.join('\n'))
  process.exit(1)
}
