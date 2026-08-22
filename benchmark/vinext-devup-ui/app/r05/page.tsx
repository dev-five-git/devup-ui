'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section03 } from '../components/Section03'
import { Section05 } from '../components/Section05'
import { Section06 } from '../components/Section06'
import { Section08 } from '../components/Section08'

export default function Route05Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={21} py={25}>
      <Flex flexDirection="column" gap={17} w="100%">
        <Text color="#22c55e" fontSize={23} fontWeight={550}>
          Route 05
        </Text>
        <Box
          bg="#8b5cf6"
          borderRadius={11}
          color="#ffffff"
          fontSize={17}
          px={15}
          py={9}
        >
          Route 05 badge
        </Box>
        <Section03 />
        <Section05 />
        <Section06 />
        <Section08 />
      </Flex>
    </Box>
  )
}
