/**
 * Ambient declarations for every CSS-in-JS package the Devup UI extractor
 * rewrites, typed as the devup-ui equivalents the code compiles to.
 *
 * The build plugins write these references into `<distDir>/compat.d.ts` for the
 * aliases they have enabled, so nothing has to be configured by hand. Reference
 * this aggregate only to opt in manually — from a tsconfig:
 *
 * ```json
 * { "compilerOptions": { "types": ["@devup-ui/react/compat"] } }
 * ```
 *
 * or from a source file:
 *
 * ```ts
 * /// <reference types="@devup-ui/react/compat" />
 * ```
 *
 * A declaration takes precedence over an installed package of the same name,
 * so pull in only the ones whose imports the build actually rewrites. The
 * per-package entries (`@devup-ui/react/compat/styled-components`, `/emotion`,
 * `/stylex`, `/vanilla-extract`) exist for exactly that.
 */

/// <reference types="@devup-ui/react/compat/styled-components" />
/// <reference types="@devup-ui/react/compat/emotion" />
/// <reference types="@devup-ui/react/compat/stylex" />
/// <reference types="@devup-ui/react/compat/vanilla-extract" />
