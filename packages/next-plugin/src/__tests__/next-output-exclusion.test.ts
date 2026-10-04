import { join } from 'node:path'

import { afterEach, expect, it } from 'bun:test'

import { planSources } from '../plan'
import { createAppContext } from '../session'
import { box, installProjectHooks, makeProject } from './project'

installProjectHooks()
const environment = { ...process.env }
afterEach(() => {
  process.env = { ...environment }
})

it('excludes actual outputs without excluding same-named source folders', () => {
  // Given
  process.env.NODE_ENV = 'production'
  const root = makeProject({
    'app/page.tsx':
      "export { C } from '../src/build/Card'; export { D } from '../src/df/Card'",
    'src/build/Card.tsx': box('color="green"'),
    'src/df/Card.tsx': box('color="blue"').replace('const C', 'const D'),
    'build/app/page.tsx': box('color="generated"'),
    'df/app/page.tsx': box('color="generated"'),
  })
  process.chdir(root)
  const context = createAppContext({ distDir: 'build' }, {})
  // When
  const plan = planSources(context)
  // Then
  expect(plan.graph?.files).toEqual([
    join(root, 'app/page.tsx'),
    join(root, 'src/build/Card.tsx'),
    join(root, 'src/df/Card.tsx'),
  ])
  expect(plan.seedFiles).toEqual(['src/build/Card.tsx', 'src/df/Card.tsx'])
})
