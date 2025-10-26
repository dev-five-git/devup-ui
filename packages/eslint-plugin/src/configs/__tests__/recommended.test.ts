import recommended from '../recommended'
describe('recommended', () => {
  it('export recommended config', () => {
    expect(recommended).toMatchSnapshot()
  })
})
