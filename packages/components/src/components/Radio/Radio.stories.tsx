import { Radio } from '.'

export default {
  title: 'Devfive/Radio',
  component: Radio,
}

export const Default = {
  args: {
    checked: undefined,
    colors: {
      primary: 'var(--primary)',
      border: 'var(--border)',
      text: 'var(--text)',
      bg: 'var(--bg)',
      hoverBg: 'var(--hoverBg)',
      hoverBorder: 'var(--hoverBorder)',
      hoverColor: 'var(--hoverColor)',
      checkedBg: 'var(--checkedBg)',
      checkedBorder: 'var(--checkedBorder)',
      checkedColor: 'var(--checkedColor)',
      disabledBg: 'var(--disabledBg)',
      disabledColor: 'var(--disabledColor)',
    },
    name: 'radio',
    children: '옵션1',
    variant: 'default',
  },
}
