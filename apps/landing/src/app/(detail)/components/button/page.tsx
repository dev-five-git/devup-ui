import { Box, Flex, Text, VStack } from '@devup-ui/react'

import { CustomCode } from '@/components/mdx/components/CustomCode'
import { CustomH4 } from '@/components/mdx/components/CustomH4'
import { CustomH6 } from '@/components/mdx/components/CustomH6'
import { CustomParagraph } from '@/components/mdx/components/CustomParagraph'
import { CustomPre } from '@/components/mdx/components/CustomPre'
import { CustomStrong } from '@/components/mdx/components/CustomStrong'

import MdxCard from '../MdxCard'
import Api from './Api.mdx'
import Button from './Button.mdx'
import { Colors } from './demo/Colors'
import { Danger } from './demo/Danger'
import { Disabled } from './demo/Disabled'
import { Icon } from './demo/Icon'
import { Variants } from './demo/Variants'

export default function Page() {
  return (
    <VStack gap="16px" maxW="100%" overflow="hidden">
      <Button
        components={{
          h4: CustomH4,
          h6: CustomH6,
          pre: CustomPre,
          strong: CustomStrong,
          p: CustomParagraph,
        }}
      />
      <VStack gap="16px" py="30px">
        <Text color="$title" typography="h6">
          Examples
        </Text>
        <Flex flexWrap="wrap" gap="16px">
          <Box flex="1" minW="300px" w="calc(50% - 8px)">
            <MdxCard demo={<Variants />} src="button/demo/Variants.tsx" />
            <MdxCard demo={<Danger />} src="button/demo/Danger.tsx" />
            <MdxCard demo={<Disabled />} src="button/demo/Disabled.tsx" />
          </Box>
          <Box flex="1" minW="300px" w="calc(50% - 8px)">
            <MdxCard demo={<Icon />} src="button/demo/Icon.tsx" />
            <MdxCard demo={<Colors />} src="button/demo/Colors.tsx" />
          </Box>
        </Flex>
      </VStack>
      <VStack gap="16px" py="30px">
        <Api
          components={{
            code: CustomCode,
            h4: CustomH4,
            h6: CustomH6,
            pre: CustomPre,
            p: CustomParagraph,
          }}
        />
      </VStack>
    </VStack>
  )
}
