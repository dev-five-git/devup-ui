/** StyleX's `styleResolution`: how a shorthand and its longhands compete. */
export const STYLEX_STYLE_RESOLUTIONS = [
  'property-specificity',
  'application-order',
  'legacy-expand-shorthands',
] as const

export type StylexStyleResolution = (typeof STYLEX_STYLE_RESOLUTIONS)[number]

/** Options for code written against StyleX's API. */
export interface StylexOptions {
  /**
   * How `stylex.create()` resolves a shorthand against its longhands, as the
   * StyleX babel plugin option of the same name (default
   * `'property-specificity'`). Devup's own APIs ignore it.
   */
  styleResolution?: StylexStyleResolution
}

export class StylexConfigError extends TypeError {
  readonly name = 'StylexConfigError'

  constructor(
    readonly path: string,
    readonly value: unknown,
    requirement: string,
  ) {
    super(
      `devup-ui option ${path} ${requirement}, got ${typeof value === 'string' ? JSON.stringify(value) : value === null ? 'null' : typeof value}`,
    )
  }
}

function isStyleResolution(value: unknown): value is StylexStyleResolution {
  return STYLEX_STYLE_RESOLUTIONS.some((resolution) => resolution === value)
}

/** The `styleResolution` the `stylex` option names, or a clear config error. */
export function normalizeStylexOptions(input: unknown): StylexStyleResolution {
  if (input === undefined) return 'property-specificity'
  if (typeof input !== 'object' || input === null || Array.isArray(input))
    throw new StylexConfigError('stylex', input, 'must be an object')
  const styleResolution: unknown = Reflect.get(input, 'styleResolution')
  if (styleResolution === undefined) return 'property-specificity'
  if (!isStyleResolution(styleResolution))
    throw new StylexConfigError(
      'stylex.styleResolution',
      styleResolution,
      `must be one of ${STYLEX_STYLE_RESOLUTIONS.map((resolution) => `'${resolution}'`).join(', ')}`,
    )
  return styleResolution
}
