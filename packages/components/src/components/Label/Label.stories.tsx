import { css } from '@devup-ui/react'
import { Meta, StoryObj } from '@storybook/react-vite'
import { useState } from 'react'
import { Label } from './index' 

type Story = StoryObj<typeof meta>

// use the same blueprint with button.
const meta: Meta<typeof Label> = {
    title: 'Devfive/Label', 
    component: Label, 
    decorators: [
        (Story) => (
            <div style={{ padding: '10px'}}>
                <Story />
            </div>
        )
    ]
}

export const Default: Story = {
    args: {
        children: 'Label Text',
        disabled: false,  
    }
}

export default meta;