import {
  collectNumberedFiles,
  enumerateProductionSourceFiles,
  type ProductionSourceFilesOptions,
} from '@devup-ui/plugin-utils'

const censusDirectories = [
  '__tests__',
  'tests',
  'test',
  '__fixtures__',
  'fixtures',
  'dist',
  'build',
  'out',
  'coverage',
  '.next',
  'tooling',
  'scripts',
] as const
const censusFile = /(?:\.(?:test|spec)\.[^./\\]+|\.d\.[mc]?ts)$/i

export function productionRootSources(
  options: ProductionSourceFilesOptions,
  include: readonly string[],
) {
  const exclude = [...(options.exclude ?? []), ...censusDirectories]
  const included = collectNumberedFiles({
    roots: [],
    include: [...include],
    exclude,
    ...(options.includeMdx === undefined
      ? {}
      : { includeMdx: options.includeMdx }),
    ...(options.cwd === undefined ? {} : { cwd: options.cwd }),
  })
  return enumerateProductionSourceFiles({
    ...options,
    sourceFiles: included,
    exclude,
  }).filter(
    ({ path, realPath }) =>
      !censusFile.test(path) && !censusFile.test(realPath),
  )
}
