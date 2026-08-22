'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section02 } from '../components/Section02'
import { Section05 } from '../components/Section05'
import { Section07 } from '../components/Section07'
import { Section08 } from '../components/Section08'

export default function Route07Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={23} py={27}>
      <Flex flexDirection="column" gap={19} w="100%">
        <Text color="#06b6d4" fontSize={25} fontWeight={570}>
          Route 07
        </Text>
        <Box
          bg="#ec4899"
          borderRadius={13}
          color="#ffffff"
          fontSize={19}
          px={17}
          py={11}
        >
          Route 07 badge
        </Box>
        <Section02 />
        <Section05 />
        <Section07 />
        <Section08 />
      </Flex>
    </Box>
  )
}
