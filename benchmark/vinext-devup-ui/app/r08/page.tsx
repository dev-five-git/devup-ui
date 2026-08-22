'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section01 } from '../components/Section01'
import { Section03 } from '../components/Section03'
import { Section06 } from '../components/Section06'
import { Section08 } from '../components/Section08'

export default function Route08Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={24} py={28}>
      <Flex flexDirection="column" gap={20} w="100%">
        <Text color="#3b82f6" fontSize={26} fontWeight={580}>
          Route 08
        </Text>
        <Box
          bg="#ef4444"
          borderRadius={14}
          color="#ffffff"
          fontSize={20}
          px={18}
          py={12}
        >
          Route 08 badge
        </Box>
        <Section01 />
        <Section03 />
        <Section06 />
        <Section08 />
      </Flex>
    </Box>
  )
}
