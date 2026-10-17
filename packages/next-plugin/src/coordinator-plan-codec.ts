export function isGenerationPlan(value: unknown): boolean {
  if (typeof value !== 'object' || value === null || Array.isArray(value))
    return false
  if (!('canonicalMap' in value) || !('expectedBaseFiles' in value))
    return false
  const map = value.canonicalMap
  return (
    typeof map === 'object' &&
    map !== null &&
    !Array.isArray(map) &&
    Object.values(map).every((bucket) => typeof bucket === 'string') &&
    Array.isArray(value.expectedBaseFiles) &&
    value.expectedBaseFiles.every((file) => typeof file === 'string')
  )
}
