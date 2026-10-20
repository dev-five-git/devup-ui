import { fileURLToPath } from 'node:url'

import { EXPECTED_EXPORTED_ROUTES } from '../../e2e/exported-routes'
import { verifyPrerender } from './verify-prerender'

verifyPrerender(
  fileURLToPath(new URL('./dist/client/', import.meta.url)),
  EXPECTED_EXPORTED_ROUTES,
)
