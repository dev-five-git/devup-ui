'use client'

import { Box, Text } from '@devup-ui/react'
import { useState } from 'react'

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
        _hover={{
          bg: 'yellow',
          cursor: 'cell',
        }}
        as="section"
        bg="$text"
        color={color}
        cursor="pointer"
        data-testid="box"
        fontSize={32}
        position="relative"
        py="28px"
      >
        <Box>hello</Box>
        <Box>hello</Box>
      </Box>
      <Text color="$text">text</Text>
      <Box color={enabled ? 'green' : 'blue'} fontSize={32} pr="20px">
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
