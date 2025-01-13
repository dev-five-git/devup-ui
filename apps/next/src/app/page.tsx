'use client'

import { Box } from '@devup-ui/react'
import { useState } from 'react'

export default function HomePage() {
  const [color, setColor] = useState('yellow')
  const [enabled, setEnabled] = useState(false)

  return (
    <div>
      <a />
      <Box
        _hover={{
          bg: 'red',
        }}
        as="span"
        bg="blue"
        color={color}
        cursor="pointer"
        data-testid="box"
        fontSize={32}
        position="relative"
        py="28px"
      >
        hello
      </Box>
      <Box color={enabled ? 'green' : 'blue'} fontSize={[32]} pr="20px">
        hello
      </Box>
      <Box fontSize={[12, 32]}>hello</Box>
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
