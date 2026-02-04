'use client'

import { Box, Button } from '@devup-ui/react'
import { useState } from 'react'

export default function HomePage() {
  const [color, setColor] = useState('text-yellow-500')
  const [enabled, setEnabled] = useState(false)

  return (
    <div>
      <Box as="p" className="bg-blue-500 text-white p-2">
        Track & field champions:
      </Box>
      <Box
        as="section"
        className={`${color} cursor-pointer relative py-7 hover:bg-yellow-500 hover:cursor-cell text-3xl`}
        data-testid="box"
      >
        <div>hello</div>
        <div>hello</div>
      </Box>
      <Box as="p" className="text-gray-900">
        text
      </Box>
      <Box
        className={`${enabled ? 'text-green-500' : 'text-blue-500'} text-3xl pr-5`}
      >
        hello
      </Box>
      <Box className="text-xs sm:text-3xl">hello</Box>
      <Button
        className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded mt-4"
        onClick={() => {
          setColor('text-blue-500')
          setEnabled((prev) => !prev)
        }}
      >
        Change
      </Button>
    </div>
  )
}
