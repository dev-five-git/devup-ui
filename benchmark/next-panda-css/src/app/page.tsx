'use client'
import { type CSSProperties, useState } from 'react'

import { Box } from '../../styled-system/jsx'

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
        color="var(--box-color)"
        cursor="pointer"
        data-testid="box"
        fontSize="32px"
        position="relative"
        py="28px"
        style={{ '--box-color': color } as CSSProperties}
      >
        <Box>hello</Box>
        <Box>hello</Box>
      </Box>
      <Box as="span" color="var(--text)">
        text
      </Box>
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
