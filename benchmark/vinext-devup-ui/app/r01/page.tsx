'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section01 } from '../components/Section01'
import { Section02 } from '../components/Section02'
import { Section04 } from '../components/Section04'
import { Section07 } from '../components/Section07'

export default function Route01Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={17} py={21}>
      <Flex flexDirection="column" gap={13} w="100%">
        <Text color="#ef4444" fontSize={19} fontWeight={510}>
          Route 01
        </Text>
        <Box
          bg="#14b8a6"
          borderRadius={7}
          color="#ffffff"
          fontSize={13}
          px={11}
          py={5}
        >
          Route 01 badge
        </Box>
        <Section01 />
        <Section02 />
        <Section04 />
        <Section07 />
      </Flex>
    </Box>
  )
}
