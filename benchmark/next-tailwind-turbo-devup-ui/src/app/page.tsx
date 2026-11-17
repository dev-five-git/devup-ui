'use client'

import { Box } from '@devup-ui/react'
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
      <Box
        as="section"
        className="bg-(--text) text-(--box-color) cursor-pointer text-[32px] relative py-7 hover:bg-[yellow] hover:cursor-cell"
        data-testid="box"
        style={{ '--box-color': color } as CSSProperties}
      >
        <div>hello</div>
        <div>hello</div>
      </Box>
      <Box as="span" className="text-(--text)">
        text
      </Box>
      <Box
        className={`${enabled ? 'text-[green]' : 'text-[blue]'} text-[32px] pr-5`}
      >
        hello
      </Box>
      <Box className="text-[12px] sm:text-[32px]">hello</Box>
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
