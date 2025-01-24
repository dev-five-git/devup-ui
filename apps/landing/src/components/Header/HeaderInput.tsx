import { Flex, Image, Input } from '@devup-ui/react'

import { URL_PREFIX } from '../../constants'

export function HeaderInput() {
  return (
    <Flex
      alignItems="center"
      bg="$menuHover"
      borderRadius="8px"
      gap="10px"
      p="8px 8px 6px"
      w="240px"
    >
      <Image boxSize="24px" src={URL_PREFIX + '/search.svg'} />
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
      />
    </Flex>
  )
}
