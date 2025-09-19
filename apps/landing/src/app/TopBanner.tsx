import { Box, Flex, Text, VStack } from '@devup-ui/react'

import { GetStartedButton } from './GetStartedButton'
import SponsorButton from './SponsorButton'
import StarButton from './StarButton'

export function TopBanner() {
  return (
    <VStack
      alignItems="center"
      bg="$backgroundLight"
      pb="100px"
      pt="200px"
      w="100%"
    >
      <VStack
        gap="40px"
        justifyContent="center"
        maxW="1440px"
        pos="relative"
        px="40px"
        w="100%"
      >
        {/* <Image
          aspectRatio="1"
          left="609px"
          pos="absolute"
          src="/icons/418625428_72f26dbd-47e8-4138-9fb0-a2e7a8fa07ff 1.png"
          top="-329px"
          w="1232px"
        /> */}
        <VStack gap="24px" justifyContent="center">
          <Text color="$title" typography="h1">
            <Text color="$primary">Zero</Text> Config
            <br />
            <Text color="$primary">Zero</Text> FOUC
            <br />
            <Text color="$primary">Zero</Text> Runtime
            <Box as="br" display={['none', null, 'initial']} />
            CSS in JS Preprocessor
          </Text>
          <Text color="$text" typography="h6Reg">
            Building the Future of CSS-in-JS
            <br />
            Analyze all possible scenarios at the fastest speed and style with
            optimal performance.
          </Text>
        </VStack>
        <GetStartedButton />
        <Flex gap="12px">
          <StarButton />
          <SponsorButton />
        </Flex>
      </VStack>
    </VStack>
  )
}
