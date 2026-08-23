'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section02 } from '../components/Section02'
import { Section03 } from '../components/Section03'
import { Section05 } from '../components/Section05'
import { Section08 } from '../components/Section08'

export default function Route02Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={18} py={22}>
      <Flex flexDirection="column" gap={14} w="100%">
        <Text color="#f97316" fontSize={20} fontWeight={520}>
          Route 02
        </Text>
        <Box
          bg="#06b6d4"
          borderRadius={8}
          color="#ffffff"
          fontSize={14}
          px={12}
          py={6}
        >
          Route 02 badge
        </Box>
        <Section02 />
        <Section03 />
        <Section05 />
        <Section08 />
      </Flex>
    </Box>
  )
}
