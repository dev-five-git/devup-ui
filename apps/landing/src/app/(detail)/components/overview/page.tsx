import { Center, Flex, Image, Text, VStack } from '@devup-ui/react'

import Card from '../Card'

export default function Page() {
  return (
    <VStack flex="1" gap="16px" maxW="100%" px="60px" py="40px">
      <Text color="$primary" typography="captionBold">
        Overview
      </Text>
      <Text color="$title" typography="h4">
        Devup UI Components
      </Text>
      <Text color="$text" typography="bodyReg">
        Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam
        venenatis, elit in hendrerit porta, augue ante scelerisque diam, ac
        egestas lacus est nec urna. Cras commodo risus hendrerit, suscipit nibh
        at, porttitor dui. Vivamus tincidunt pretium nibh et pulvinar. Nam quis
        tristique neque, vitae facilisis justo. Ut non tristique dui.
      </Text>
      <VStack gap="16px" overflow="visible" py="30px">
        <Text color="$title" typography="h6">
          Form
        </Text>
        <Flex alignItems="center" flexWrap="wrap" gap="20px" overflow="visible">
          <Card>
            <Center h="140px">
              <Image src="/images/components/button.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Button
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/text-box.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Text Box
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/text-area.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Text area
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/dropdown.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Dropdown
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/radio.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Radio
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/checkbox.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Checkbox
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/stepper.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Stepper
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/toggle.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Toggle
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/slider.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Slider
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/date-picker.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Date picker
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/color-picker.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Color picker
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/uploader.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Uploader
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/pagination.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Pagination
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/progress-bar.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Progress bar
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/search.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Search
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/select.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Select
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/label.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Label
              </Text>
            </Flex>
          </Card>
        </Flex>
      </VStack>
      <VStack gap="16px" overflow="visible" py="30px">
        <Text color="$title" typography="h6">
          Layout
        </Text>
        <Flex alignItems="center" flexWrap="wrap" gap="20px" overflow="visible">
          <Card>
            <Center h="140px">
              <Image src="/images/components/layout/footer.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Footer
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/layout/tooltip.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Tooltip
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/layout/tab.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Tab
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/layout/menu.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Menu
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/layout/header.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Header
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/layout/confirm.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Confirm
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/layout/snackbar.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Snackbar
              </Text>
            </Flex>
          </Card>
          <Card>
            <Center h="140px">
              <Image src="/images/components/layout/bottom-sheet.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Bottom sheet
              </Text>
            </Flex>
          </Card>
        </Flex>
      </VStack>
      <VStack gap="16px" overflow="visible" py="30px">
        <Text color="$title" typography="h6">
          Theme
        </Text>
        <Flex alignItems="center" flexWrap="wrap" gap="20px" overflow="visible">
          <Card>
            <Center h="140px">
              <Image src="/images/components/theme/theme-button.svg" />
            </Center>
            <Flex
              alignItems="center"
              borderTop="1px solid $border"
              gap="10px"
              px="16px"
              py="12px"
            >
              <Text color="$text" textAlign="right" typography="buttonSmid">
                Theme Button
              </Text>
            </Flex>
          </Card>
        </Flex>
      </VStack>
    </VStack>
  )
}
