'use client'

import { Box, Typography } from '@mui/material'
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
        component="section"
        data-testid="box"
        sx={{
          color,
          cursor: 'pointer',
          fontSize: 32,
          position: 'relative',
          py: '28px',
          '&:hover': {
            bg: 'yellow',
            cursor: 'cell',
          },
        }}
      >
        <Box>hello</Box>
        <Box>hello</Box>
      </Box>
      <Typography color="$text">text</Typography>
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
