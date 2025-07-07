import { render } from '@testing-library/react'
import { expect } from 'vitest'

import { DevupTheme } from '../../types/theme'
import { ThemeScript } from '../ThemeScript'

describe('ThemeScript', () => {
  it('should apply ThemeScript', () => {
    vi.stubEnv('DEVUP_UI_DEFAULT_THEME', undefined)
    const { container } = render(<ThemeScript />)
    expect(container).toMatchSnapshot()
    vi.unstubAllEnvs()
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
