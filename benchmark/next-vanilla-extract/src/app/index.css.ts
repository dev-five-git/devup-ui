import { style } from '@vanilla-extract/css'

export const hello = style({
  cursor: 'pointer',
  fontSize: 32,
  position: 'relative',
  paddingTop: '28px',
  paddingBottom: '28px',
  selectors: {
    '&:hover': {
      backgroundColor: 'yellow',
      cursor: 'cell',
    },
  },
})

export const text = style({
  color: 'var(--text)',
})

export const conditional = style({
  color: 'green',
})

export const conditional1 = style({
  color: 'blue',
})

export const hello3 = style({
  paddingRight: '20px',
  fontSize: 32,
})

export const hello2 = style({
  fontSize: 12,
})
