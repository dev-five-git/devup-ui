'use client'

import { Box, Center, Flex, Text, VStack } from '@devup-ui/react'
import { useState } from 'react'

import IconCode from '@/components/icons/IconCode'

export default function MdxCardFooter({
  children,
}: {
  children: React.ReactNode
}) {
  const [isOpen, setIsOpen] = useState(false)
  return (
    <>
      <VStack justifyContent="flex-end" maxH="600px" maxW="100%">
        <Flex
          borderTop="1px solid $border"
          justifyContent="flex-end"
          px="24px"
          py="10px"
          w="100%"
        >
          <Center
            _active={{
              bg: '$menuActive',
            }}
            _hover={{
              bg: '$menuHover',
            }}
            borderRadius="4px"
            color="$captionBold"
            cursor="pointer"
            gap="8px"
            onClick={() => setIsOpen(!isOpen)}
            p="8px"
            transition="all 0.2s ease-in-out"
            w="fit-content"
          >
            <IconCode isOpen={isOpen} />
            <Text>Show Code</Text>
          </Center>
        </Flex>
        {isOpen && (
          <Box
            borderTop="1px solid $border"
            maxW="100%"
            overflow="auto"
            px="24px"
            py="32px"
          >
            {children}
          </Box>
        )}
      </VStack>
    </>
  )
}
