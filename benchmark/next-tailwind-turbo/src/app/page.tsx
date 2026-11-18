'use client'

import { type CSSProperties, useState } from 'react'

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
      <section
        className="bg-(--text) text-(--box-color) cursor-pointer text-[32px] relative py-7 hover:bg-[yellow] hover:cursor-cell"
        data-testid="box"
        style={{ '--box-color': color } as CSSProperties}
      >
        <div>hello</div>
        <div>hello</div>
      </section>
      <span className="text-(--text)">text</span>
      <div
        className={`${enabled ? 'text-[green]' : 'text-[blue]'} text-[32px] pr-5`}
      >
        hello
      </div>
      <div className="text-[12px] min-[480px]:text-[32px]">hello</div>
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
