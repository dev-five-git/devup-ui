import { Box, Flex, Image, Text, VStack } from '@devup-ui/react'

import { GetStartedButton } from './GetStartedButton'
import SponsorButton from './SponsorButton'
import StarButton from './StarButton'

export function TopBanner() {
  return (
    <VStack
      _themeDark={{
        bg: 'linear-gradient(180deg, #29304F 0%, #1B141C 100%)',
      }}
      alignItems="center"
      bg="linear-gradient(180deg, #E1E5F5 0%, #FEF4FF 100%)"
      h={['100lvh', null, '840px', null, '800px']}
      overflow="hidden"
      pb="100px"
      pos="relative"
      pt={['90px', null, '250px', null, '200px']}
      w="100%"
    >
      <Image
        _themeDark={{
          opacity: 0.8,
          mixBlendMode: 'overlay',
        }}
        bottom={['-150px', null, 'auto']}
        boxSize={['510px', null, '1230px']}
        pos="absolute"
        right={['50%', null, '-400px', null, '-160px']}
        src="/top-banner.webp"
        top={['auto', null, '-80px', null, '-130px']}
        transform={['translateX(50%)', null, 'none']}
      />
      <VStack
        alignItems={['center', null, 'flex-start']}
        gap="40px"
        justifyContent="center"
        maxW="1440px"
        pos="relative"
        px={[4, null, '40px']}
        w="100%"
      >
        <VStack
          gap="24px"
          justifyContent="center"
          textAlign={['center', null, 'left']}
        >
          <Text color="$title" px={[6, null, 0]} typography="h1">
            <Text color="$primary">Zero</Text> Config
            <br />
            <Text color="$primary">Zero</Text> FOUC
            <br />
            <Text color="$primary">Zero</Text> Runtime{' '}
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
