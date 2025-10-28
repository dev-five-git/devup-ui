'use client'

import { Box, Center, Flex, Grid, Image, Text, VStack } from '@devup-ui/react'
import { useState } from 'react'

import IconCode from '@/components/icons/IconCode'

export default function MdxCardFooter({
  code,
  children,
}: {
  code: string
  children: React.ReactNode
}) {
  const [isOpen, setIsOpen] = useState(false)
  const [copied, setCopied] = useState(false)

  const handleCopy = () => {
    navigator.clipboard
      .writeText(code)
      .then(() => {
        setCopied(true)
        setTimeout(() => {
          setCopied(false)
        }, 1000 * 5)
      })
      .catch(() => setCopied(false))
  }

  return (
    <VStack justifyContent="flex-end" maxW="100%">
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
          <IconCode open={isOpen} />
          <Text>Show Code</Text>
        </Center>
      </Flex>
      <Grid
        gridTemplateRows={isOpen ? '1fr' : '0fr'}
        maxH="600px"
        pos="relative"
        transition="grid-template-rows 0.3s ease-in-out"
      >
        {isOpen && (
          <>
            <Box pos="absolute" w="100%">
              <Center
                _active={{
                  borderColor: '$primary',
                  bg: '$menuActive',
                }}
                _hover={{
                  borderColor: '$primary',
                  bg: '$menuHover',
                }}
                bg="$containerBackground"
                border="1px solid transparent"
                borderRadius="4px"
                boxShadow="0 2px 6px 0 $shadow"
                cursor="pointer"
                gap="8px"
                onClick={handleCopy}
                p="8px"
                pos="absolute"
                right="16px"
                top="16px"
                transition="all 0.125s ease-in-out"
              >
                <Image
                  aspectRatio="1"
                  boxSize="20px"
                  src={copied ? '/icons/copied.svg' : '/icons/copy-code.svg'}
                />
                <Text color="$captionBold" typography="caption">
                  Copy
                </Text>
              </Center>
            </Box>
          </>
        )}
        <Box
          borderTop={isOpen ? '1px solid $border' : 'none'}
          h="100%"
          onWheel={(e) => e.stopPropagation()}
          overflow="auto"
          p={isOpen ? ['12px', null, '24px'] : '0'}
          transition="padding 0.2s ease-in-out"
        >
          {children}
        </Box>
      </Grid>
    </VStack>
  )
}
