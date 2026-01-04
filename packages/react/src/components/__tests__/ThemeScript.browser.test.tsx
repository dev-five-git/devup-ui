import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import { DevupTheme } from '../../types/theme'
import { ThemeScript } from '../ThemeScript'

describe('ThemeScript', () => {
  it('should apply ThemeScript', () => {
    const { container } = render(<ThemeScript />)
    expect(container).toMatchSnapshot()
  })
  it('should apply ThemeScript with theme', () => {
    const { container } = render(
      <ThemeScript theme={'default' as keyof DevupTheme} />,
    )
    expect(container).toMatchSnapshot()
  })
  it('should apply ThemeScript with not auto', () => {
    const { container } = render(<ThemeScript auto={false} />)
    expect(container).toMatchSnapshot()
  })
})
