import { expect, it } from 'bun:test'

import { createAppContext, createSession } from '../session'
import { createTurboRules, SOURCE_RULE } from '../turbo-rules'

it.each(['src/componentmdx.tsx', 'src/page.mdx.tsx'])(
  'extracts an ordinary source whose real filename is %s',
  (filename) => {
    // Given
    const context = createAppContext({}, {})
    const rules = createTurboRules({
      context,
      session: createSession(context),
      themeFiles: [],
      theme: {},
    })
    // When
    const rule = rules[SOURCE_RULE]
    const condition = rule && !Array.isArray(rule) ? rule.condition : undefined
    if (
      !condition ||
      typeof condition !== 'object' ||
      !('not' in condition) ||
      typeof condition.not !== 'object' ||
      !('path' in condition.not) ||
      !(condition.not.path instanceof RegExp)
    ) {
      throw new TypeError('Expected a path exclusion condition')
    }
    // Then
    expect(condition.not.path.test(filename)).toBe(false)
  },
)
