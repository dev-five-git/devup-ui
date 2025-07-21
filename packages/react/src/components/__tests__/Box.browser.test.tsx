// @ts-nocheck
import { Box } from '@devup-ui/react'
import { render, waitFor } from '@testing-library/react'

describe('Box', () => {
  it('should render', async () => {
    const { container } = render(<Box bg="blue" />)

    await waitFor(
      () => {
        expect(container.children[0]).toHaveClass('background-0-blue--255')
      },
      {
        timeout: 1000,
        interval: 10,
      },
    )
  })
})
