'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section01 } from '../components/Section01'
import { Section02 } from '../components/Section02'
import { Section04 } from '../components/Section04'
import { Section07 } from '../components/Section07'

export default function Route09Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={25} py={29}>
      <Flex flexDirection="column" gap={21} w="100%">
        <Text color="#6366f1" fontSize={27} fontWeight={590}>
          Route 09
        </Text>
        <Box
          bg="#f97316"
          borderRadius={15}
          color="#ffffff"
          fontSize={21}
          px={19}
          py={13}
        >
          Route 09 badge
        </Box>
        <Section01 />
        <Section02 />
        <Section04 />
        <Section07 />
      </Flex>
    </Box>
  )
}
