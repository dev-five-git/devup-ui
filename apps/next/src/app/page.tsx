'use client'

import { Box, css, styled, Text } from '@devup-ui/react'
import { useState } from 'react'
import { Lib } from 'vite-lib-example'
const color = 'yellow'

const StyledFooter = styled.footer<{ type: '1' | '2' }>`
  background-color: ${color};
  color: ${(props) => (props.type === '1' ? 'red' : 'white')};
`

export default function HomePage() {
  const [color, setColor] = useState('yellow')
  const [enabled, setEnabled] = useState(false)

  return (
    <div>
      <StyledFooter type="2">IMPLEMENTATION~</StyledFooter>
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
        <Lib />
        <Box>hello</Box>
      </Box>
      <Text
        className={css`
          background: red;
          color: blue;
        `}
        typography="bold"
      >
        text typo
      </Text>
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
