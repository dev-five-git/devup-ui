import { resetCss } from '../index'
vi.mock('@devup-ui/react', () => ({
  globalCss: vi.fn(),
}))

describe('reset-css', () => {
  it('should be defined', () => {
    expect(resetCss).toBeInstanceOf(Function)
    expect(resetCss()).toBeUndefined()
  })
})
