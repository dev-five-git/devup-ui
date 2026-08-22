'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section01 } from '../components/Section01'
import { Section04 } from '../components/Section04'
import { Section06 } from '../components/Section06'
import { Section07 } from '../components/Section07'

export default function Route06Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={22} py={26}>
      <Flex flexDirection="column" gap={18} w="100%">
        <Text color="#14b8a6" fontSize={24} fontWeight={560}>
          Route 06
        </Text>
        <Box
          bg="#d946ef"
          borderRadius={12}
          color="#ffffff"
          fontSize={18}
          px={16}
          py={10}
        >
          Route 06 badge
        </Box>
        <Section01 />
        <Section04 />
        <Section06 />
        <Section07 />
      </Flex>
    </Box>
  )
}
