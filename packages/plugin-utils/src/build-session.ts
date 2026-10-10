// The self-reference stays external in both published entry bundles. Bun source
// tests and consumers select the same internal module, as do Node import/require.
import admission from '@devup-ui/plugin-utils/internal/build-admission'

export const {
  beginBuild,
  MixedBuildIntegrationError,
  resetOwnedBuildState,
  runBuildOperation,
} = admission

export type {
  BuildIntegration,
  ResettableEngine,
} from '@devup-ui/plugin-utils/internal/build-admission'
