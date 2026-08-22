'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section02 } from '../components/Section02'
import { Section03 } from '../components/Section03'
import { Section05 } from '../components/Section05'
import { Section08 } from '../components/Section08'

export default function Route10Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={26} py={30}>
      <Flex flexDirection="column" gap={22} w="100%">
        <Text color="#8b5cf6" fontSize={28} fontWeight={600}>
          Route 10
        </Text>
        <Box
          bg="#f59e0b"
          borderRadius={16}
          color="#ffffff"
          fontSize={22}
          px={20}
          py={14}
        >
          Route 10 badge
        </Box>
        <Section02 />
        <Section03 />
        <Section05 />
        <Section08 />
      </Flex>
    </Box>
  )
}
