'use client'

import { Box, css, Text } from '@devup-ui/react'
import { useState } from 'react'

export default function HomePage() {
  const [color, setColor] = useState('yellow')
  const [enabled, setEnabled] = useState(false)

  return (
    <div>
      <a />
      <Box
        _hover={{
          bg: ['yellow', 'red'],
          cursor: 'cell',
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
      <Text
        className={css`
          background: red;
          color: blue;
        `}
      >
        text
      </Text>
      <Text color="$text">text</Text>
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
