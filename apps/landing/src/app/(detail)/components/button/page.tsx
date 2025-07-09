import { Box, css, Text, VStack } from '@devup-ui/react'

import Api from './Api.mdx'
import Button from './Button.mdx'
import Examples from './Examples.mdx'

function CustomParagraph({ children }: { children: React.ReactNode }) {
  return (
    <Text as="p" color="$text" lineHeight="1" m="0" typography="bodyReg">
      {children}
    </Text>
  )
}

function CustomPre({ children }: { children: React.ReactNode }) {
  return (
    <Box
      as="pre"
      className={css({
        margin: '0',
        w: '100%',
        whiteSpace: 'pre-wrap',
        lineBreak: 'anywhere',
        bg: 'transparent',
        overflow: 'auto',
      })}
      selectors={{
        '& pre': {
          margin: '0',
          w: '100%',
          whiteSpace: 'pre-wrap',
          lineBreak: 'anywhere',
          bg: 'transparent',
          overflow: 'auto',
        },
        '& pre, & code, & span, & p': {
          margin: '0',
          w: '100%',
          whiteSpace: 'pre-wrap',
          lineBreak: 'anywhere',
          bg: 'transparent',
          overflow: 'auto',
        },
      }}
    >
      {children}
    </Box>
  )
}

function CustomCode({ children }: { children: string }) {
  return (
    <Box as="code" color="$primary" whiteSpace="pre-wrap">
      {children.replaceAll('<br>', '\n')}
    </Box>
  )
}

function CustomH4({ children }: { children: React.ReactNode }) {
  return (
    <Text as="h4" color="$title" m="0" typography="h4">
      {children}
    </Text>
  )
}

function CustomH6({ children }: { children: React.ReactNode }) {
  return (
    <Text as="h6" color="$title" m="0" typography="h6">
      {children}
    </Text>
  )
}

function CustomStrong({ children }: { children: React.ReactNode }) {
  return (
    <Text as="strong" color="$primary" m="0" typography="captionBold">
      {children}
    </Text>
  )
}

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
        <Examples
          components={{
            h4: CustomH4,
            h6: CustomH6,
            pre: CustomPre,
            p: CustomParagraph,
          }}
        />
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
