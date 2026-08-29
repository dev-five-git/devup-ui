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

  it('should escape a closing script tag in the theme', () => {
    const { container } = render(
      <ThemeScript theme={'</script><script>' as keyof DevupTheme} />,
    )

    expect(container.querySelector('script')?.textContent).toContain(
      '\\u003c/script>',
    )
    expect(container.querySelectorAll('script')).toHaveLength(1)
  })
})
