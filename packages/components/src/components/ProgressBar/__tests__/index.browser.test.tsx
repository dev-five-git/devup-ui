import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'
import { createRef } from 'react'

import { ProgressBar } from '../index'

describe('ProgressBar', () => {
  it('renders a bar and a step progress', () => {
    expect(render(<ProgressBar value={40} />).container).toMatchSnapshot()
    expect(
      render(<ProgressBar max={6} showValue value={3} variant="step" />)
        .container,
    ).toMatchSnapshot()
  })

  it('is a progressbar with its range and name', () => {
    const { getByRole } = render(
      <ProgressBar aria-label="Upload" max={200} value={50} />,
    )
    const bar = getByRole('progressbar', { name: 'Upload' })

    expect(bar).toHaveAttribute('aria-valuemin', '0')
    expect(bar).toHaveAttribute('aria-valuemax', '200')
    expect(bar).toHaveAttribute('aria-valuenow', '50')
  })

  it('fills the bar to the share of max', () => {
    const { container } = render(<ProgressBar max={200} showValue value={50} />)
    const indicator = container.querySelector('[role=progressbar] > div > div')

    expect(indicator?.getAttribute('style')).toContain('25%')
    expect(container.querySelector('[aria-hidden]')?.textContent).toBe('25%')
  })

  it('rounds the shown percent', () => {
    const { container } = render(<ProgressBar max={3} showValue value={1} />)

    expect(container.querySelector('[aria-hidden]')?.textContent).toBe('33%')
  })

  it('fills one segment per finished step', () => {
    const { container } = render(
      <ProgressBar max={6} showValue value={3} variant="step" />,
    )

    expect(
      container.querySelectorAll('[role=progressbar] > div > div'),
    ).toHaveLength(6)
    expect(container.querySelectorAll('[data-filled]')).toHaveLength(3)
    expect(container.querySelector('[aria-hidden]')?.textContent).toBe('3/6')
  })

  it('counts only whole steps', () => {
    const { container, getByRole } = render(
      <ProgressBar max={4.7} showValue value={2.5} variant="step" />,
    )

    expect(getByRole('progressbar')).toHaveAttribute('aria-valuemax', '4')
    expect(container.querySelectorAll('[data-filled]')).toHaveLength(2)
    expect(container.querySelector('[aria-hidden]')?.textContent).toBe('2/4')
  })

  it.each([
    [-10, 100, '0'],
    [150, 100, '100'],
    [Number.NaN, 100, '0'],
  ])('keeps %p within 0 and %p', (value, max, now) => {
    const { getByRole } = render(<ProgressBar max={max} value={value} />)

    expect(getByRole('progressbar')).toHaveAttribute('aria-valuenow', now)
  })

  it.each([0, -3, Number.NaN])('treats a max of %p as 1', (max) => {
    const { getByRole } = render(<ProgressBar max={max} value={1} />)

    expect(getByRole('progressbar')).toHaveAttribute('aria-valuemax', '1')
  })

  it('hides the value unless asked', () => {
    const { container } = render(<ProgressBar value={10} />)

    expect(container.querySelector('[aria-hidden]')).toBeNull()
  })

  it('takes class names, colors, native props and a ref', () => {
    const ref = createRef<HTMLDivElement>()
    const { container } = render(
      <ProgressBar
        ref={ref}
        className="outer"
        classNames={{ track: 'track', indicator: 'indicator', value: 'value' }}
        colors={{ primary: 'red', track: 'gray', text: 'blue' }}
        data-testid="progress"
        id="progress"
        max={2}
        showValue
        value={1}
        variant="step"
      />,
    )

    expect(ref.current?.id).toBe('progress')
    expect(ref.current).toHaveClass('outer')
    expect(ref.current).toHaveAttribute('data-testid', 'progress')
    expect(ref.current?.getAttribute('style')).toContain('--primary: red')
    expect(ref.current?.getAttribute('style')).toContain('--track: gray')
    expect(ref.current?.getAttribute('style')).toContain('--text: blue')
    expect(container.querySelector('.indicator')).toHaveAttribute('data-filled')
    expect(container.querySelector('.track')).not.toHaveAttribute('data-filled')
    expect(container.querySelector('.value')?.textContent).toBe('1/2')
  })

  it('gives the bar its class names', () => {
    const { container } = render(
      <ProgressBar
        classNames={{ track: 'track', indicator: 'indicator' }}
        value={10}
      />,
    )

    expect(container.querySelector('.track > .indicator')).toBeInTheDocument()
  })
})
