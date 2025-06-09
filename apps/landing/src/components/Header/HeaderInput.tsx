import { Flex, Image, Input } from '@devup-ui/react'

export function HeaderInput(props: React.ComponentProps<'input'>) {
  return (
    <Flex
      alignItems="center"
      bg="$menuHover"
      borderRadius="8px"
      gap="10px"
      p="8px 8px 6px"
      w="100%"
    >
      <Image boxSize="24px" src="/search.svg" />
      <Input
        _placeholder={{
          color: '$caption',
        }}
        bg="transparent"
        border="none"
        color="$text"
        outline="none"
        placeholder="Search documentation..."
        typography="caption"
        w="100%"
        {...props}
      />
    </Flex>
  )
}
