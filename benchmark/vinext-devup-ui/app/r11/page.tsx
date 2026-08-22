'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section01 } from '../components/Section01'
import { Section03 } from '../components/Section03'
import { Section04 } from '../components/Section04'
import { Section06 } from '../components/Section06'

export default function Route11Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={27} py={31}>
      <Flex flexDirection="column" gap={23} w="100%">
        <Text color="#d946ef" fontSize={29} fontWeight={610}>
          Route 11
        </Text>
        <Box
          bg="#84cc16"
          borderRadius={17}
          color="#ffffff"
          fontSize={23}
          px={21}
          py={15}
        >
          Route 11 badge
        </Box>
        <Section01 />
        <Section03 />
        <Section04 />
        <Section06 />
      </Flex>
    </Box>
  )
}
