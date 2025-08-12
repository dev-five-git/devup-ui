import { Meta, StoryObj } from '@storybook/react-vite'

import {
  Stepper,
  StepperContainer,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
} from './index'

type Story = StoryObj<typeof meta>

// More on how to set up stories at: https://storybook.js.org/docs/writing-stories#default-export
const meta: Meta<typeof Stepper> = {
  title: 'Devfive/Stepper',
  component: Stepper,
  decorators: [
    (Story) => (
      <div style={{ padding: '10px' }}>
        <Story />
      </div>
    ),
  ],
}

export const Default: Story = {
  args: {},
  render: (args) => (
    <Stepper {...args}>
      <StepperContainer>
        <StepperDecreaseButton />
        <StepperInput editable={false} />
        <StepperIncreaseButton />
      </StepperContainer>
    </Stepper>
  ),
}

export default meta
