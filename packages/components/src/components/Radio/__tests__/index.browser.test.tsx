import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import { Radio } from '../index'

describe('Radio', () => {
  it('should Radio snapshot', () => {
    expect(render(<Radio />).container).toMatchSnapshot()
    expect(render(<Radio variant="button" />).container).toMatchSnapshot()
    expect(render(<Radio disabled />).container).toMatchSnapshot()
    expect(
      render(<Radio disabled variant="button" />).container,
    ).toMatchSnapshot()
    expect(
      render(
        <Radio
          disabled
          style={{
            width: '500px',
          }}
        />,
      ).container,
    ).toMatchSnapshot()
    expect(
      render(
        <Radio
          disabled
          style={{
            width: '500px',
          }}
          styles={{
            label: {
              width: '500px',
            },
          }}
          variant="button"
        />,
      ).container,
    ).toMatchSnapshot()
    expect(
      render(
        <Radio
          disabled
          style={{
            width: '500px',
          }}
          styles={{
            label: {
              width: '500px',
            },
          }}
        />,
      ).container,
    ).toMatchSnapshot()

    expect(
      render(
        <Radio
          className="className"
          classNames={{ label: 'classNameLabel' }}
          disabled
          variant="button"
        />,
      ).container,
    ).toMatchSnapshot()
    expect(
      render(
        <Radio
          className="className"
          classNames={{ label: 'classNameLabel' }}
          disabled
        />,
      ).container,
    ).toMatchSnapshot()

    expect(
      render(<Radio firstButton variant="button" />).container,
    ).toMatchSnapshot()
    expect(
      render(<Radio lastButton variant="button" />).container,
    ).toMatchSnapshot()
  })
})
