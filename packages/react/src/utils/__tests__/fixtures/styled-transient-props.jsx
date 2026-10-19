import { createRequire } from 'node:module'
import process from 'node:process'

import { styled } from '@devup-ui/react'

// Bun's isolated install lets a package resolve only what it declares, and
// `react-dom` is a peer of the DOM preload's `@testing-library/react`
const domPreload = createRequire(import.meta.url).resolve('bun-test-env-dom')
const testingLib = createRequire(domPreload).resolve('@testing-library/react')
const server = createRequire(testingLib)('react-dom/server')

const recorded = []

function Recorder(props) {
  recorded.push(props)
  return null
}

/**
 * Server-renders `Component` with `props` spread at runtime by a caller the
 * build cannot tie to it, like one in another module, giving the markup, what
 * React wrote to `console.error` and the props `Recorder` received. React
 * warns about an invalid attribute name once per process, so each tag
 * scenario uses its own `$` names.
 */
function renderWith(Component, props) {
  const warnings = []
  const { error } = console
  console.error = (...args) => {
    warnings.push(args.join(' '))
  }
  recorded.length = 0
  try {
    return {
      html: server.renderToStaticMarkup(<Component {...props} />),
      received: [...recorded],
      warnings,
    }
  } finally {
    console.error = error
  }
}

const ordinary = { 'data-testid': 'target', id: 'target', title: 'Target' }
const transient = { $: 'bare', $id: 'transient-id', $other: 'other' }

const Title = styled.h1`
  color: ${(p) => p.$tone};
`
const Wrapped = styled(Recorder)`
  color: red;
`
const Forwarding = styled.h1.withConfig({ shouldForwardProp: () => true })`
  color: red;
`
const Attributed = styled.h1.attrs({ $fromAttrs: 'attrs' })`
  color: red;
`
const Heading = styled.h1`
  color: red;
`

// What each scenario observed, for the test that runs this script
process.stdout.write(
  JSON.stringify({
    tag: renderWith(Title, { ...ordinary, ...transient, $tone: 'teal' }),
    component: renderWith(Wrapped, { ...ordinary, ...transient }),
    forwarding: renderWith(Forwarding, { ...ordinary, $forced: 'forced' }),
    attrs: renderWith(Attributed, ordinary),
    as: renderWith(Heading, { ...ordinary, ...transient, as: Recorder }),
  }),
)
