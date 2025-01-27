import { Box, Flex, Image, Text, VStack } from '@devup-ui/react'

import { URL_PREFIX } from '../../../constants'

export function RightIndex() {
  return (
    <VStack gap="16px" p="20px 16px" w="200px">
      <VStack>
        <Flex alignItems="center" gap="10px" p="6px 0px">
          <Text color="$text" flex="1" typography="captionBold">
            Contents
          </Text>
        </Flex>
        <Flex alignItems="center" gap="10px" p="4px 10px">
          <Box bg="$primary" borderRadius="100%" boxSize="6px" />
          <Text
            color="$primary"
            flex="1"
            opacity="0.8"
            typography="captionBold"
          >
            Installation
          </Text>
        </Flex>
        <Flex alignItems="center" gap="10px" p="4px 10px 30px">
          <Text color="$text" flex="1" opacity="0.6" typography="caption">
            General Guides
          </Text>
        </Flex>
        <Flex alignItems="center" gap="10px" p="4px 10px 30px">
          <Text color="$text" flex="1" opacity="0.6" typography="caption">
            Framework Guides
          </Text>
        </Flex>
        <Flex alignItems="center" gap="10px" p="4px 10px">
          <Text color="$text" flex="1" opacity="0.6" typography="caption">
            Next Steps
          </Text>
        </Flex>
        <Flex alignItems="center" gap="10px" p="4px 10px">
          <Text color="$text" flex="1" opacity="0.6" typography="caption">
            Playground
          </Text>
        </Flex>
        <Flex alignItems="center" gap="10px" p="4px 10px">
          <Text color="$text" flex="1" opacity="0.6" typography="caption">
            Acknowledgement
          </Text>
        </Flex>
      </VStack>
      <Box bg="undefined" h="1px" />
      <Flex gap="4px">
        <Text color="$caption" flex="1" textAlign="right" typography="small">
          Edit this page
        </Text>
        <Image
          bg="$caption"
          boxSize="16px"
          maskImage={`url(${URL_PREFIX}/outlink.svg)`}
          maskSize="100%"
        />
      </Flex>
    </VStack>
  )
}
