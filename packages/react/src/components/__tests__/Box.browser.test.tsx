import { Box } from '@devup-ui/react'
import { describe, expect, it } from 'bun:test'

describe('Box', () => {
  it('should render', () => {
    expect(<Box bg="blue" />).toHaveClass('background-0-blue--255')
  })
})
