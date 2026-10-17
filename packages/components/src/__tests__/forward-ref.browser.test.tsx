import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'
import { createRef } from 'react'

import { Button, Checkbox, Input, Radio, Textarea } from '../index'

describe('forwardRef', () => {
  it('links both labels to the forwarded checkbox when a custom id is given', () => {
    // Given
    const ref = createRef<HTMLInputElement>()

    // When
    const { container } = render(
      <Checkbox ref={ref} id="custom-checkbox">
        Checkbox
      </Checkbox>,
    )

    // Then
    expect(ref.current?.id).toBe('custom-checkbox')
    const labels = container.querySelectorAll('label')
    expect(labels).toHaveLength(2)
    for (const label of labels) {
      expect(label.htmlFor).toBe(ref.current?.id)
    }
  })

  it('hands its ref to the native radio when the button variant is used', () => {
    // Given
    const ref = createRef<HTMLInputElement>()

    // When
    render(
      <Radio ref={ref} variant="button">
        Radio
      </Radio>,
    )

    // Then
    expect(ref.current?.type).toBe('radio')
  })

  it('hands each component ref to its form element', () => {
    const button = createRef<HTMLButtonElement>()
    const input = createRef<HTMLInputElement>()
    const radio = createRef<HTMLInputElement>()
    const textarea = createRef<HTMLTextAreaElement>()
    render(
      <>
        <Button ref={button}>b</Button>
        <Input ref={input} />
        <Radio ref={radio}>r</Radio>
        <Textarea ref={textarea} />
      </>,
    )
    expect(button.current?.tagName).toBe('BUTTON')
    expect(input.current?.tagName).toBe('INPUT')
    expect(radio.current?.type).toBe('radio')
    expect(textarea.current?.tagName).toBe('TEXTAREA')
  })
})
