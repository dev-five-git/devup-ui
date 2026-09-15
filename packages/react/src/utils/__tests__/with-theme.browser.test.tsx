import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import type { StyledTheme } from '../theme-vars'
import { withTheme } from '../with-theme'

describe('withTheme', () => {
  it('injects the css-variable theme accessor', () => {
    const Swatch = ({ theme }: { theme?: StyledTheme }) => (
      <div>{`${(theme as { brand: string }).brand}`}</div>
    )
    const Themed = withTheme(Swatch)
    const { container } = render(<Themed />)
    expect(container.textContent).toBe('var(--brand)')
  })
})
