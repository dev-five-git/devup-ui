import { Center, css, Flex, Grid, Text, VStack } from '@devup-ui/react'

import * as Icons from '@/components/icons/components'

import Card from '../Card'

export default function Page() {
  return (
    <VStack gap="16px" px={['16px', '30px', '60px']} py={['24px', '40px']}>
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
        <Grid
          gap={['10px', null, null, null, '20px']}
          gridTemplateColumns={[
            'repeat(1, 1fr)',
            'repeat(3, 1fr)',
            null,
            'repeat(4, 1fr)',
            'repeat(5, 1fr)',
          ]}
        >
          <Card>
            <Center h="140px">
              <Icons.IconButtonComponent className={css({ w: '100%' })} />
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
              <Icons.IconTextBoxComp className={css({ w: '100%' })} />
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
              <Icons.IconTextAreaComp className={css({ w: '100%' })} />
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
              <Icons.IconDropdownComp className={css({ w: '100%' })} />
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
              <Icons.IconRadioComp className={css({ w: '100%' })} />
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
              <Icons.IconCheckboxComp className={css({ w: '100%' })} />
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
              <Icons.IconStepperComp className={css({ w: '100%' })} />
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
              <Icons.IconToggleComp className={css({ w: '100%' })} />
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
              <Icons.IconSliderComp className={css({ w: '100%' })} />
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
              <Icons.IconDatePickerComp className={css({ w: '100%' })} />
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
              <Icons.IconColorPickerComp className={css({ w: '100%' })} />
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
              <Icons.IconUploaderComp className={css({ w: '100%' })} />
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
              <Icons.IconPagination className={css({ w: '100%' })} />
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
              <Icons.IconProgressBar className={css({ w: '100%' })} />
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
              <Icons.IconSearchComp className={css({ w: '100%' })} />
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
              <Icons.IconSelectComp className={css({ w: '100%' })} />
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
              <Icons.IconLabelComp className={css({ w: '100%' })} />
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
        </Grid>
      </VStack>
      <VStack gap="16px" overflow="visible" py="30px">
        <Text color="$title" typography="h6">
          Layout
        </Text>
        <Grid
          gap={['10px', null, null, null, '20px']}
          gridTemplateColumns={[
            'repeat(1, 1fr)',
            'repeat(3, 1fr)',
            null,
            'repeat(4, 1fr)',
            'repeat(5, 1fr)',
          ]}
          overflow="visible"
        >
          <Card>
            <Center h="140px">
              <Icons.IconFooterComp className={css({ w: '100%' })} />
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
              <Icons.IconTooltipComp className={css({ w: '100%' })} />
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
              <Icons.IconTabComp className={css({ w: '100%' })} />
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
              <Icons.IconMenuComp className={css({ w: '100%' })} />
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
              <Icons.IconHeaderComp className={css({ w: '100%' })} />
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
              <Icons.IconConfirmComp className={css({ w: '100%' })} />
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
              <Icons.IconSnackbarComp className={css({ w: '100%' })} />
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
              <Icons.IconBottomSheetComp className={css({ w: '100%' })} />
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
        </Grid>
      </VStack>
      <VStack gap="16px" overflow="visible" py="30px">
        <Text color="$title" typography="h6">
          Theme
        </Text>
        <Grid
          gap={['10px', null, null, null, '20px']}
          gridTemplateColumns={[
            'repeat(1, 1fr)',
            'repeat(3, 1fr)',
            null,
            'repeat(4, 1fr)',
            'repeat(5, 1fr)',
          ]}
          overflow="visible"
        >
          <Card>
            <Center h="140px">
              <Icons.IconThemeButtonComp className={css({ w: '100%' })} />
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
        </Grid>
      </VStack>
    </VStack>
  )
}
