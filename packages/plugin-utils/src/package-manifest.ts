export class PackageManifestShapeError extends TypeError {
  readonly name = 'PackageManifestShapeError'

  constructor(readonly manifestPath: string) {
    super(`Package manifest must contain a JSON object: ${manifestPath}`)
  }
}
