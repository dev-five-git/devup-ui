import type { Conditional } from './utils'

/* eslint-disable @typescript-eslint/no-empty-object-type */
// biome-ignore lint/suspicious/noEmptyInterface: public module augmentation point for user typography tokens.
export interface DevupThemeTypography {}

export type DevupThemeTypographyKeys = Conditional<DevupThemeTypography>
