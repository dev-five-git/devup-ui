'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section02 } from '../components/Section02'
import { Section04 } from '../components/Section04'
import { Section05 } from '../components/Section05'
import { Section07 } from '../components/Section07'

export default function Route12Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={28} py={32}>
      <Flex flexDirection="column" gap={24} w="100%">
        <Text color="#ec4899" fontSize={30} fontWeight={620}>
          Route 12
        </Text>
        <Box
          bg="#22c55e"
          borderRadius={18}
          color="#ffffff"
          fontSize={24}
          px={22}
          py={16}
        >
          Route 12 badge
        </Box>
        <Section02 />
        <Section04 />
        <Section05 />
        <Section07 />
      </Flex>
    </Box>
  )
}
