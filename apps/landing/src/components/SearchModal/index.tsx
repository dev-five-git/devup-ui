import { VStack } from '@devup-ui/react'
import { Suspense } from 'react'

import { SearchContent } from './SearchContent'
import { SearchInput } from './SearchInput'
import { SearchModalWrap } from './SearchModalWrap'

export function SearchModal() {
  return (
    <Suspense>
      <SearchModalWrap>
        <VStack
          bg="$containerBackground"
          borderBottomRadius={[null, null, '16px']}
          borderTopRadius="16px"
          bottom={['0', null, 'auto']}
          gap="10px"
          left={['0', null, '50%']}
          maxH="600px"
          maxW={[null, null, '532px']}
          p="16px"
          pos="absolute"
          top={[null, null, '40%']}
          transform={[null, null, 'translate(-50%, -50%)']}
          w="100%"
        >
          <SearchInput />
          <SearchContent />
        </VStack>
      </SearchModalWrap>
    </Suspense>
  )
}
