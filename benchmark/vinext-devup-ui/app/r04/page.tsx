'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section02 } from '../components/Section02'
import { Section04 } from '../components/Section04'
import { Section05 } from '../components/Section05'
import { Section07 } from '../components/Section07'

export default function Route04Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={20} py={24}>
      <Flex flexDirection="column" gap={16} w="100%">
        <Text color="#84cc16" fontSize={22} fontWeight={540}>
          Route 04
        </Text>
        <Box
          bg="#6366f1"
          borderRadius={10}
          color="#ffffff"
          fontSize={16}
          px={14}
          py={8}
        >
          Route 04 badge
        </Box>
        <Section02 />
        <Section04 />
        <Section05 />
        <Section07 />
      </Flex>
    </Box>
  )
}
