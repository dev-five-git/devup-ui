import { Button } from '@devup-ui/components'
import { Box, css, Flex } from '@devup-ui/react'

/**
 * **Disabled**
 * Use `disabled` prop to show disabled UI of the button.
 */
export function Disabled() {
  return (
    <Box w="100%">
      <Flex flexWrap="wrap" gap="12px" mb="16px">
        <Button
          className={css({ height: 'min-content' })}
          disabled
          variant="primary"
        >
          Primary disabled
        </Button>
        <Button className={css({ height: 'min-content' })} disabled>
          Default disabled
        </Button>
      </Flex>
    </Box>
  )
}
