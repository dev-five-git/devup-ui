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
      default: 'blue',
      ':hover': 'yellow',
    },
    cursor: {
      default: 'pointer',
      ':hover': 'cell',
    },
  },
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
    fontSize: 12,
  },
})

export default function HomePage() {
  const [_, setColor] = useState('yellow')
  const [enabled, setEnabled] = useState(false)

  return (
    <div>
      <Box as="p" style={{ backgroundColor: 'blue' }}>
        Track & field champions:
      </Box>
      <Box as="section" {...stylex.props(styles.base)}>
        <div>hello</div>
        <div>hello</div>
      </Box>
      <Box {...stylex.props(styles.typo)}>text</Box>
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
