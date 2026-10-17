export function parseSourceMap(sourceMap: string | undefined): string | null {
  if (!sourceMap) return null
  JSON.parse(sourceMap)
  return sourceMap
}

export function parseCoordinatorResponse(content: string) {
  const data: unknown = JSON.parse(content)
  if (
    typeof data !== 'object' ||
    data === null ||
    !('code' in data) ||
    typeof data.code !== 'string'
  )
    throw new Error('Coordinator response missing code')
  const resolution: unknown =
    'resolutionInputs' in data
      ? data.resolutionInputs
      : { fileDependencies: [], missingDependencies: [] }
  if (
    typeof resolution !== 'object' ||
    resolution === null ||
    !('fileDependencies' in resolution) ||
    !('missingDependencies' in resolution)
  )
    throw new TypeError('Coordinator response has invalid resolution inputs')
  const files = resolution.fileDependencies
  const missing = resolution.missingDependencies
  if (
    !Array.isArray(files) ||
    files.some((path: unknown) => typeof path !== 'string') ||
    !Array.isArray(missing) ||
    missing.some((path: unknown) => typeof path !== 'string')
  )
    throw new TypeError('Coordinator response has invalid resolution paths')
  return Object.freeze({
    code: data.code,
    map: parseSourceMap(
      'map' in data && typeof data.map === 'string' ? data.map : undefined,
    ),
    dependencies:
      'dependencies' in data && Array.isArray(data.dependencies)
        ? data.dependencies.filter(
            (dependency): dependency is string =>
              typeof dependency === 'string',
          )
        : [],
    missingDependencies: Object.freeze(
      missing.filter((path): path is string => typeof path === 'string'),
    ),
    fileDependencies: Object.freeze(
      files.filter((path): path is string => typeof path === 'string'),
    ),
  })
}
