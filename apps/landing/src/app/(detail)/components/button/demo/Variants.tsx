import { Button } from '@devup-ui/components'
import { css, Flex } from '@devup-ui/react'

/**
 * **Variant & Size**
 * `Button` components has `default` and `primary` variants. `Size` prop determines the paddings of the button.
 */
export function Variants() {
  return (
    <>
      <Flex flexWrap="wrap" gap="12px" mb="16px">
        <Button
          className={css({ h: 'min-content' })}
          size="sm"
          variant="primary"
        >
          Primary sm
        </Button>
        <Button
          className={css({ h: 'min-content' })}
          size="md"
          variant="primary"
        >
          Primary md
        </Button>
        <Button
          className={css({ h: 'min-content' })}
          size="lg"
          variant="primary"
        >
          Primary lg
        </Button>
      </Flex>
      <Flex flexWrap="wrap" gap="12px" mb="16px">
        <Button className={css({ h: 'min-content' })} size="sm">
          Default sm
        </Button>
        <Button className={css({ h: 'min-content' })} size="md">
          Default md
        </Button>
        <Button className={css({ h: 'min-content' })} size="lg">
          Default lg
        </Button>
      </Flex>
    </>
  )
}
