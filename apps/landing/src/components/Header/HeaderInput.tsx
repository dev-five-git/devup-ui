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
        bg="transparent"
        border="none"
        color="$text"
        outline="none"
        placeholder="Search documentation..."
        w="100%"
        {...props}
        _placeholder={{
          color: '$caption',
        }}
        typography="caption"
      />
    </Flex>
  )
}
