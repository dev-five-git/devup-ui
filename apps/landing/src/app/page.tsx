import { Box, css, Flex, Grid, Image, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'

import { CodeBoard } from '../components/CodeBoard'
import { Container } from '../components/Container'
import { Discord } from '../components/Discord'
import { URL_PREFIX } from '../constants'
import { FeatureCard } from './FeatureCard'

export default function HomePage() {
  return (
    <>
      <Box pt={['100px', '150px']}>
        <VStack alignItems="center" gap="50px" maxW="800px" mx="auto">
          <VStack alignItems="center" gap="24px">
            <Image h="50px" src={URL_PREFIX + '/icon.svg'} w="52px" />
            <Text color="$title" textAlign="center" typography="h1">
              Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor
            </Text>
            <Text color="$text" textAlign="center" typography="h6Reg">
              Building the Future of CSS-in-JS
              <br />
              Analyze all possible scenarios at the fastest speed and style with
              optimal performance.
            </Text>
          </VStack>
          <Link
            className={css({
              textDecoration: 'none',
            })}
            href={URL_PREFIX + '/docs/overview'}
          >
            <Flex
              _active={{
                bg: '$negativeBase',
              }}
              _hover={{
                bg: '$title',
              }}
              alignItems="center"
              bg="$text"
              borderRadius="100px"
              gap="20px"
              p="16px 40px"
              role="group"
            >
              <Box
                _groupActive={{
                  bg: '$third',
                }}
                _groupHover={{
                  bg: '$primary',
                }}
                bg="$secondary"
                borderRadius="100%"
                boxSize="10px"
              />
              <Flex alignItems="center" gap="10px">
                <Text color="$base" typography="buttonL">
                  Get started
                </Text>
                <Image
                  bg="$base"
                  boxSize="24px"
                  maskImage={`url(${URL_PREFIX + '/arrow.svg'})`}
                />
              </Flex>
            </Flex>
          </Link>
        </VStack>
        <Box maxW="1224px" mx="auto" pb="100px" pt="80px" px={3}>
          <CodeBoard />
        </Box>
        <VStack
          gap="40px"
          maxW="1232px"
          mx="auto"
          pb={[4, 10, '120px']}
          pt={[4, null, '40px']}
          px={4}
        >
          <VStack gap="16px">
            <Text color="$title" typography="h4">
              Features
            </Text>
            <Text color="$text" typography="textL">
              Devup UI offers a performance-optimized CSS-in-JS system, theme
              typing, and amazing features for faster and safer development.
            </Text>
          </VStack>
          <Grid gap="16px" gridTemplateColumns={['1fr', null, '1fr 1fr']}>
            <FeatureCard
              description="A futuristic design that eliminates the root causes of performance degradation."
              icon="/idea.svg"
              title="Zero Runtime"
            />
            <FeatureCard
              description="The fastest build speed and the smallest bundle size among CSS-in-JS solutions."
              icon="/trophy.svg"
              title="Top Performance"
            />
            <FeatureCard
              description="Enhanced DX with typing-based support."
              icon="/heart.svg"
              title="Type Safety"
            />
            <FeatureCard
              description="A Figma plugin enabling safer and faster development."
              icon="/notice.svg"
              title="Figma Plugin"
            />
          </Grid>
        </VStack>
        <Container>
          <Box px={4} py="40px">
            <Discord />
          </Box>
        </Container>
      </Box>
    </>
  )
}
