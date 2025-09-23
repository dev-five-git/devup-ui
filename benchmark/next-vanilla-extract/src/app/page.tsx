'use client'

import { useState } from 'react'

import {
  conditional,
  conditional1,
  hello,
  hello2,
  hello3,
  text,
} from './index.css'

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
      <section className={hello}>
        <div>hello</div>
        <div>hello</div>
      </section>
      <p className={text}>text</p>
      <div className={`${enabled ? conditional : conditional1} ${hello3}`}>
        hello
      </div>
      <div className={hello2}>hello</div>
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
