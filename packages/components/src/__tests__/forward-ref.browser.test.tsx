import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'
import { createRef } from 'react'

import { Button, Input, Radio, Textarea } from '../index'

describe('forwardRef', () => {
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
