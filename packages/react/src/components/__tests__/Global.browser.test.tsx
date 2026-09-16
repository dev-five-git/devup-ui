import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import { Global } from '../Global'

describe('Global', () => {
  it('renders nothing because the styles are extracted at build time', () => {
    const { container } = render(<Global styles={{ body: { margin: 0 } }} />)
    expect(container.innerHTML).toBe('')
  })
})
