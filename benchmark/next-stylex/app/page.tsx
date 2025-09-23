'use client'

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
      <p
        style={{
          backgroundColor: 'blue',
        }}
      >
        Track & field champions:
      </p>
      <section {...stylex.props(styles.base)}>
        <div>hello</div>
        <div>hello</div>
      </section>
      <div {...stylex.props(styles.typo)}>text</div>
      <div
        {...stylex.props(
          enabled ? styles.conditional : styles.conditional1,
          styles.hello,
        )}
      >
        hello
      </div>
      <div {...stylex.props(styles.hello2)}>hello</div>
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
