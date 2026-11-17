'use client'

import { Box } from '@devup-ui/react'
import * as stylex from '@stylexjs/stylex'
import { useState } from 'react'

const styles = stylex.create({
  base: {
    fontSize: 32,
    position: 'relative',
    paddingTop: '28px',
    paddingBottom: '28px',
    backgroundColor: {
      default: 'var(--text)',
      ':hover': 'yellow',
    },
    cursor: {
      default: 'pointer',
      ':hover': 'cell',
    },
  },
  dynamicColor: (color: string) => ({
    color,
  }),
  typo: {
    color: 'var(--text)',
  },
  conditional: {
    color: 'green',
  },
  conditional1: {
    color: 'blue',
  },
  hello: {
    fontSize: 32,
    paddingRight: '20px',
  },
  hello2: {
    fontSize: {
      default: 12,
      '@media (min-width: 480px)': 32,
    },
  },
})

export default function HomePage() {
  const [color, setColor] = useState('yellow')
  const [enabled, setEnabled] = useState(false)

  return (
    <div>
      <p
        style={{
          backgroundColor: 'blue',
        }}
      >
        Track & field champions:
      </p>
      <Box
        as="section"
        {...stylex.props(styles.base, styles.dynamicColor(color))}
        data-testid="box"
      >
        <div>hello</div>
        <div>hello</div>
      </Box>
      <Box as="span" {...stylex.props(styles.typo)}>
        text
      </Box>
      <Box
        {...stylex.props(
          enabled ? styles.conditional : styles.conditional1,
          styles.hello,
        )}
      >
        hello
      </Box>
      <Box {...stylex.props(styles.hello2)}>hello</Box>
      <button
        onClick={() => {
          setColor('blue')
          setEnabled((prev) => !prev)
        }}
      >
        Change
      </button>
    </div>
  )
}
