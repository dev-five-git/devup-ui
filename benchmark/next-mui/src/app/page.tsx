'use client'

import { Box } from '@mui/material'
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
          bgcolor: 'var(--text)',
          color,
          cursor: 'pointer',
          fontSize: 32,
          position: 'relative',
          py: '28px',
          '&:hover': {
            bgcolor: 'yellow',
            cursor: 'cell',
          },
        }}
      >
        <Box>hello</Box>
        <Box>hello</Box>
      </Box>
      <Box component="span" sx={{ color: 'var(--text)' }}>
        text
      </Box>
      <Box sx={{ color: enabled ? 'green' : 'blue', fontSize: 32, pr: '20px' }}>
        hello
      </Box>
      <Box sx={{ fontSize: 12, '@media (min-width: 480px)': { fontSize: 32 } }}>
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
