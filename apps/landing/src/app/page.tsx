import { Box, css, Flex, Image, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'

import { CodeBoard } from '../components/CodeBoard'
import { Container } from '../components/Container'
import { Discord } from '../components/Discord'
import { Header } from '../components/Header'
import { URL_PREFIX } from '../constants'

export default function HomePage() {
  return (
    <>
      <Header />
      <Box mt="150px">
        <VStack alignItems="center" gap="50px" maxW="800px" mx="auto">
          <VStack alignItems="center" gap="24px">
            <Image h="50px" src={URL_PREFIX + '/icon.svg'} w="52px" />
            <Text color="$title" textAlign="center" typography="h1">
              Lorem ipsum dolor sit amet,
              <br />
              consectetur adipiscing elit.
            </Text>
            <Text textAlign="center" typography="h6Reg">
              Etiam sit amet feugiat turpis. Proin nec ante a sem vestibulum
              sodales non ut ex. Morbi diam turpis, fringilla vitae enim et,
              egestas consequat nibh. Etiam auctor cursus urna sit amet
              elementum.
            </Text>
          </VStack>
          <Link
            className={css`
              text-decoration: none;
            `}
            href={URL_PREFIX + '/docs/overview'}
          >
            <Flex
              alignItems="center"
              bg="$text"
              borderRadius="100px"
              gap="20px"
              p="16px 40px"
            >
              <Box bg="$secondary" borderRadius="100%" boxSize="10px" />
              <Flex alignItems="center" gap="10px">
                <Text color="$base" typography="buttonL">
                  Get started
                </Text>
                <Image boxSize="24px" src={URL_PREFIX + '/arrow.svg'} />
              </Flex>
            </Flex>
          </Link>
        </VStack>
        <Box maxW="1200px" mx="auto" pb="100px" pt="80px">
          <CodeBoard />
        </Box>
        <VStack gap="40px" maxW="1200px" mx="auto" pb="120px" pt="40px">
          <VStack gap="16px" w="805px">
            <Text color="$title" typography="h4">
              Lorem ipsum dolor sit amet.
            </Text>
            <Text typography="textL">
              Etiam sit amet feugiat turpis. Proin nec ante a sem vestibulum
              sodales non ut ex. Morbi diam turpis, fringilla vitae enim et,
              egestas consequat nibh.
            </Text>
          </VStack>
          <VStack gap="16px">
            <Flex alignItems="center" gap="16px">
              <Flex
                bg="$cardBg"
                borderRadius="20px"
                flex="1"
                gap="10px"
                p="24px"
              >
                <Flex px="8px">
                  <Image boxSize="32px" src={URL_PREFIX + '/idea.svg'} />
                </Flex>
                <VStack flex="1" gap="10px">
                  <Text color="$title" typography="h6">
                    Feature title
                  </Text>
                  <Text flex="1" typography="body" w="484px">
                    Lorem ipsum dolor sit amet.
                  </Text>
                </VStack>
              </Flex>
              <Flex
                bg="$cardBg"
                borderRadius="20px"
                flex="1"
                gap="10px"
                p="30px 24px"
              >
                <Flex px="8px">
                  <Image boxSize="32px" src={URL_PREFIX + '/trophy.svg'} />
                </Flex>
                <VStack flex="1" gap="10px">
                  <Text color="$title" typography="h6">
                    Feature title
                  </Text>
                  <Text flex="1" typography="body" w="484px">
                    Lorem ipsum dolor sit amet. Etiam sit amet feugiat turpis.
                    Proin nec ante a sem vestibulum sodales non ut ex.
                  </Text>
                </VStack>
              </Flex>
            </Flex>
            <Flex alignItems="center" gap="16px">
              <Flex
                bg="$cardBg"
                borderRadius="20px"
                flex="1"
                gap="10px"
                p="30px 24px"
              >
                <Flex px="8px">
                  <Image boxSize="32px" src={URL_PREFIX + '/heart.svg'} />
                </Flex>
                <VStack flex="1" gap="10px">
                  <Text color="$title" typography="h6">
                    Feature title
                  </Text>
                  <Text flex="1" typography="body" w="484px">
                    Lorem ipsum dolor sit amet. Etiam sit amet feugiat turpis.
                    Proin nec ante a sem vestibulum.
                  </Text>
                </VStack>
              </Flex>
              <Flex
                bg="$cardBg"
                borderRadius="20px"
                flex="1"
                gap="10px"
                p="30px 24px"
              >
                <Flex px="8px">
                  <Image boxSize="32px" src={URL_PREFIX + '/notice.svg'} />
                </Flex>
                <VStack flex="1" gap="10px">
                  <Text color="$title" typography="h6">
                    Feature title
                  </Text>
                  <Text flex="1" typography="body" w="484px">
                    Lorem ipsum dolor sit amet. Etiam sit amet feugiat turpis.
                  </Text>
                </VStack>
              </Flex>
            </Flex>
          </VStack>
        </VStack>
        <Container>
          <Box py="40px">
            <Discord />
          </Box>
        </Container>
      </Box>
    </>
  )
}
