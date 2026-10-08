import {
  Button,
  Checkbox,
  Input,
  Radio,
  Select,
  SelectContainer,
  SelectOption,
  SelectTrigger,
  Stepper,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
  Textarea,
} from '@devup-ui/components'
import { Box, styled } from '@devup-ui/react'
import { useEffect, useRef, useState, version } from 'react'

const Field = styled('input', { color: 'red', p: 1 })

export function App() {
  const buttonRef = useRef<HTMLButtonElement>(null)
  const checkboxRef = useRef<HTMLInputElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const radioRef = useRef<HTMLInputElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const fieldRef = useRef<HTMLInputElement>(null)
  const [refs, setRefs] = useState('')
  const [selected, setSelected] = useState('')

  useEffect(() => {
    setRefs(
      [buttonRef, checkboxRef, inputRef, radioRef, textareaRef, fieldRef]
        .map((ref) => ref.current?.tagName.toLowerCase() ?? 'null')
        .join(','),
    )
  }, [])

  return (
    <Box p={4}>
      <output data-testid="version">{version}</output>
      <Button ref={buttonRef}>button</Button>
      <Checkbox ref={checkboxRef}>checkbox</Checkbox>
      <Input ref={inputRef} />
      <Radio ref={radioRef}>radio</Radio>
      <Textarea ref={textareaRef} />
      <Field ref={fieldRef} />
      <output data-testid="refs">{refs}</output>
      <Select onChange={(value) => setSelected(String(value))}>
        <SelectTrigger>select</SelectTrigger>
        <SelectContainer>
          <SelectOption value="first">first</SelectOption>
          <SelectOption value="second">second</SelectOption>
        </SelectContainer>
      </Select>
      <output data-testid="selected">{selected}</output>
      <Stepper>
        <StepperDecreaseButton />
        <StepperInput data-testid="stepper" />
        <StepperIncreaseButton data-testid="increase" />
      </Stepper>
    </Box>
  )
}
