import { Box, Flex, Image, Text, VStack } from '@devup-ui/react'

export function Bench() {
  return (
    <VStack alignItems="center" py="60px" w="100%">
      <VStack gap="30px" maxW="1440px" px="40px" w="100%">
        <VStack gap="16px" w="805px">
          <Text color="$title" typography="h4">
            Comparison Bechmarks
          </Text>
          <Text color="$text" typography="textL">
            Next.js Build Time and Build Size (github action - ubuntu-latest)
          </Text>
        </VStack>
        <Flex alignItems="center" gap="20px">
          <VStack
            bg="$containerBackground"
            border="1px solid $border"
            borderRadius="20px"
            boxShadow="0 0 8px 0 var(--shadow, rgba(135, 135, 135, 0.25))"
            gap="60px"
            justifyContent="center"
            overflow="hidden"
            p="30px"
            pos="relative"
            w="400px"
          >
            <Image
              aspectRatio="1"
              left="-60px"
              opacity="0.2"
              pos="absolute"
              src="/icons/Group 1.svg"
              top="132px"
              w="260px"
            />
            <Image
              aspectRatio="1"
              bottom="-30px"
              opacity="0.2"
              pos="absolute"
              right="200px"
              src="/icons/Group 2.svg"
              w="260px"
            />
            <VStack gap="8px">
              <Text color="$text" typography="h5">
                Devup-ui
              </Text>
              <Text color="$text" typography="textL">
                1.0.15
              </Text>
            </VStack>
            <VStack alignItems="flex-end" gap="20px">
              <VStack alignItems="flex-end" gap="6px" justifyContent="center">
                <Text color="$text" typography="textSbold">
                  Next.js Build TIme{' '}
                </Text>
                <Flex gap="10px">
                  <Box aspectRatio="1" overflow="hidden" w="24px">
                    <Box
                      bg="#FFC100"
                      h="14.548px"
                      maskImage="url(/icons/Vector.svg)"
                      maskRepeat="no-repeat"
                      maskSize="contain"
                      w="22.8px"
                    />
                  </Box>
                  <Text
                    bg="linear-gradient(270deg, #6BB1F2 0%, #8235CA 100%)"
                    typography="h4"
                  >
                    16.8s
                  </Text>
                </Flex>
              </VStack>
              <VStack alignItems="flex-end" gap="6px" justifyContent="center">
                <Text color="$text" typography="textSbold">
                  Bulid Size
                </Text>
                <Flex gap="10px">
                  <Box aspectRatio="1" overflow="hidden" w="24px">
                    <Box
                      bg="#FFC100"
                      h="14.548px"
                      maskImage="url(/icons/Vector.svg)"
                      maskRepeat="no-repeat"
                      maskSize="contain"
                      w="22.8px"
                    />
                  </Box>
                  <Text
                    bg="linear-gradient(270deg, #6BB1F2 0%, #8235CA 100%)"
                    typography="h4"
                  >
                    51.2MB
                  </Text>
                </Flex>
              </VStack>
            </VStack>
          </VStack>
          <VStack
            bg="$cardBg"
            borderRadius="20px"
            flex="1"
            gap="40px"
            justifyContent="center"
            p="30px"
          >
            <VStack gap="8px">
              <Text color="$captionBold" typography="h6">
                Chakra UI
              </Text>
              <Text color="$captionBold" typography="textL">
                3.24.2
              </Text>
            </VStack>
            <VStack alignItems="flex-end" gap="20px">
              <VStack alignItems="flex-end" gap="6px" justifyContent="center">
                <Text color="$captionBold" typography="textSbold">
                  Bulid Time
                </Text>
                <Text color="$caption" typography="h5">
                  29.3s
                </Text>
              </VStack>
              <VStack alignItems="flex-end" gap="6px" justifyContent="center">
                <Text color="$captionBold" typography="textSbold">
                  Bulid Size
                </Text>
                <Text color="$caption" typography="h5">
                  186.2MB
                </Text>
              </VStack>
            </VStack>
          </VStack>
          <VStack
            bg="$cardBg"
            borderRadius="20px"
            flex="1"
            gap="40px"
            justifyContent="center"
            p="30px"
          >
            <VStack gap="8px">
              <Text color="$captionBold" typography="h6">
                Mui
              </Text>
              <Text color="$captionBold" typography="textL">
                7.3.1
              </Text>
            </VStack>
            <VStack alignItems="flex-end" gap="20px">
              <VStack alignItems="flex-end" gap="6px" justifyContent="center">
                <Text color="$captionBold" typography="textSbold">
                  Bulid Time
                </Text>
                <Text color="$caption" typography="h5">
                  21.6s
                </Text>
              </VStack>
              <VStack alignItems="flex-end" gap="6px" justifyContent="center">
                <Text color="$captionBold" typography="textSbold">
                  Bulid Size
                </Text>
                <Text color="$caption" typography="h5">
                  84.3MB
                </Text>
              </VStack>
            </VStack>
          </VStack>
          <VStack
            bg="$cardBg"
            borderRadius="20px"
            flex="1"
            gap="40px"
            justifyContent="center"
            p="30px"
          >
            <VStack gap="8px">
              <Text color="$captionBold" typography="h6">
                Kuma UI
              </Text>
              <Text color="$captionBold" typography="textL">
                1.5.9
              </Text>
            </VStack>
            <VStack alignItems="flex-end" gap="20px">
              <VStack alignItems="flex-end" gap="6px" justifyContent="center">
                <Text color="$captionBold" typography="textSbold">
                  Bulid Time
                </Text>
                <Text color="$caption" typography="h5">
                  20.6s
                </Text>
              </VStack>
              <VStack alignItems="flex-end" gap="6px" justifyContent="center">
                <Text color="$captionBold" typography="textSbold">
                  Bulid Size
                </Text>
                <Text color="$caption" typography="h5">
                  60.3B
                </Text>
              </VStack>
            </VStack>
          </VStack>
        </Flex>
      </VStack>
    </VStack>
  )
}
