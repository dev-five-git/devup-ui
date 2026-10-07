'use client'

import { Box, Flex, globalCss } from '@devup-ui/react'
import { useState } from 'react'

import { badge } from './styles.mjs'

globalCss({ body: { margin: 0 } })

export function Fixture() {
  const [color, setColor] = useState('#123456')
  return (
    <Flex as="main" data-testid="layout" flexDir="column" gap="12px">
      <Box data-testid="static" bg="#2468ac" p={4} borderRadius="7px">
        Static extraction
      </Box>
      <Box data-testid="theme" color="$primary" px="$gutter">
        Extended theme
      </Box>
      <Box data-testid="responsive" w={['100px', '200px']}>
        Responsive extraction
      </Box>
      <Box as="button" data-testid="hover" bg="#112233" _hover={{ bg: '#abcdef' }}>
        Hover extraction
      </Box>
      <Box data-testid="dynamic" bg={color}>
        Dynamic CSS variable
      </Box>
      <button onClick={() => setColor('#654321')}>Change value</button>
      <div data-testid="imported" className={badge}>Imported module</div>
    </Flex>
  )
}
