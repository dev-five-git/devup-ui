import { Toggle } from './index'

export default {
  title: 'Devfive/Toggle',
  component: Toggle,
  tags: ['autodocs'],
}

export const Default = {
  args: {
    disabled: false,
    variant: 'default',
    defaultValue: false,
    offText: 'OFF',
    onText: 'ON',
    color: 'var(--primary)',
  },
}
