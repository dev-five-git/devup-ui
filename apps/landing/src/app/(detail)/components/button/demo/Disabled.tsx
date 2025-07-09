import { Button } from '@devup-ui/components'
import { Box, css } from '@devup-ui/react'

/**
 * **Disabled**
 * Use `disabled` prop to show disabled UI of the button.
 */
export function Disabled() {
  return (
    <Box width="100%">
      <Box display="flex" flexWrap="wrap" gap="12px" marginBottom="16px">
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
      </Box>
    </Box>
  )
}
