'use client'

import { Box, Flex, Text } from '@devup-ui/react'

import { Section01 } from '../components/Section01'
import { Section03 } from '../components/Section03'
import { Section04 } from '../components/Section04'
import { Section06 } from '../components/Section06'

export default function Route03Page() {
  return (
    <Box bg="#f8fafc" minH="100vh" px={19} py={23}>
      <Flex flexDirection="column" gap={15} w="100%">
        <Text color="#f59e0b" fontSize={21} fontWeight={530}>
          Route 03
        </Text>
        <Box
          bg="#3b82f6"
          borderRadius={9}
          color="#ffffff"
          fontSize={15}
          px={13}
          py={7}
        >
          Route 03 badge
        </Box>
        <Section01 />
        <Section03 />
        <Section04 />
        <Section06 />
      </Flex>
    </Box>
  )
}
