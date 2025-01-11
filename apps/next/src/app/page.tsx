'use client'

import { Box } from '@devup-ui/react'
import { useState } from 'react'

export default function HomePage() {
  const [color, setColor] = useState('yellow')
  const [enabled, setEnabled] = useState(false)

  return (
    <div>
      <Box bg="red" color={color} cursor="pointer" fontSize={32}>
        hello
      </Box>
      <Box color={enabled ? 'red' : 'blue'} fontSize={32}>
        hello
      </Box>
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
