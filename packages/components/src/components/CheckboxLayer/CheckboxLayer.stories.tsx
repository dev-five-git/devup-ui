import { Meta, StoryObj } from '@storybook/react-vite'

import { CheckboxLayer } from '.'

type Story = StoryObj<typeof meta>

const meta: Meta<typeof CheckboxLayer> = {
  title: 'Devfive/CheckboxLayer',
  component: CheckboxLayer,
  decorators: [
    (Story) => (
      <div style={{ padding: '20px' }}>
        <Story />
      </div>
    ),
  ],
  argTypes: {
    onCheckboxChange: { action: 'checkbox changed' },
  },
}

export const RowLayout: Story = {
  args: {
    checkboxes: [
      { id: 'option1', value: '옵션 1 값', label: '옵션 1' },
      {
        id: 'option2',
        value: (
          <span style={{ color: 'blue', fontWeight: 'bold' }}>
            파란색 텍스트
          </span>
        ),
        label: '옵션 2',
      },
      {
        id: 'option3',
        value: (
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
            <span>🎉</span>
            <span>이모지와 텍스트</span>
          </div>
        ),
        label: '옵션 3',
      },
      { id: 'option4', value: 42, label: '옵션 4', disabled: true },
      {
        id: 'option5',
        value: (
          <button style={{ padding: '4px 8px', borderRadius: '4px' }}>
            버튼 요소
          </button>
        ),
        label: '옵션 5',
        disabled: true,
        checked: true,
      },
    ],
    flexDir: 'row',
    defaultCheckedIds: ['option2', 'option5'], // 체크됨, disabled and checked
    onCheckboxChange: (event) => {
      console.info('체크박스 변경됨:', event)
      console.info(
        `ID: ${event.id}, Value: ${event.value}, Checked: ${event.checked}`,
      )
      console.info('전체 선택된 값들:', event.checkedValues)
    },
  },
}

export const ColumnLayout: Story = {
  args: {
    checkboxes: [
      { id: 'option1', value: '옵션 1 값', label: '옵션 1' },
      {
        id: 'option2',
        value: (
          <span style={{ color: 'blue', fontWeight: 'bold' }}>
            파란색 텍스트
          </span>
        ),
        label: '옵션 2',
      },
      {
        id: 'option3',
        value: (
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
            <span>🎉</span>
            <span>이모지와 텍스트</span>
          </div>
        ),
        label: '옵션 3',
      },
      { id: 'option4', value: 42, label: '옵션 4', disabled: true },
      {
        id: 'option5',
        value: (
          <button style={{ padding: '4px 8px', borderRadius: '4px' }}>
            버튼 요소
          </button>
        ),
        label: '옵션 5',
        disabled: true,
        checked: true,
      },
    ],
    flexDir: 'column',
    defaultCheckedIds: ['option2', 'option5'], // 체크됨, disabled and checked
    onCheckboxChange: (event) => {
      console.info('체크박스 변경됨:', event)
      console.info(
        `ID: ${event.id}, Value: ${event.value}, Checked: ${event.checked}`,
      )
      console.info('전체 선택된 값들:', event.checkedValues)
    },
  },
}

export default meta
