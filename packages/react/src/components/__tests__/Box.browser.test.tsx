// @ts-nocheck
import { Box } from '@devup-ui/react'
import { render } from '@testing-library/react'

describe('Box', () => {
  it('should render', () => {
    const { container } = render(<Box bg="blue" />)
    expect(container.children[0]).toHaveStyle('background-color: blue')
  })
})
