'use client'

import { Box, Text } from '@chakra-ui/react'
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
        bg="var(--text)"
        color={color}
        cursor="pointer"
        data-testid="box"
        fontSize="32px"
        position="relative"
        py="28px"
      >
        <Box>hello</Box>
        <Box>hello</Box>
      </Box>
      <Text as="span" color="var(--text)">
        text
      </Text>
      <Box color={enabled ? 'green' : 'blue'} fontSize="32px" pr="20px">
        hello
      </Box>
      <Box fontSize={['12px', '32px']}>hello</Box>
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
