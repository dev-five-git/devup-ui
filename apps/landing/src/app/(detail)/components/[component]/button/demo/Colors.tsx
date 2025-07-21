import { Button } from '@devup-ui/components'
import { Box, css, Flex } from '@devup-ui/react'

/**
 * **Colors**
 * Pass in an object containing color tokens into `colors` prop. You can change color of border, background, danger color and more using `primary`, `error`, `text`, and so on.
 */
export default function Colors() {
  return (
    <Box w="100%">
      <Flex flexWrap="wrap" gap="12px" mb="16px">
        <Button
          className={css({ h: 'min-content' })}
          colors={{
            primary: 'orange',
            text: 'chocolate',
          }}
        >
          Default
        </Button>
        <Button
          className={css({ h: 'min-content' })}
          colors={{
            primary: 'orange',
            text: 'chocolate',
          }}
          variant="primary"
        >
          Primary
        </Button>
        <Button
          className={css({ h: 'min-content' })}
          colors={{
            error: 'orange',
            text: 'chocolate',
          }}
          danger
        >
          Danger
        </Button>
      </Flex>
      <Flex flexWrap="wrap" gap="12px" mb="16px">
        <Button
          className={css({ h: 'min-content' })}
          colors={{
            primary: 'darkgreen',
            text: 'darkseagreen',
          }}
        >
          Default
        </Button>
        <Button
          className={css({ h: 'min-content' })}
          colors={{
            primary: 'darkgreen',
            text: 'darkseagreen',
          }}
          variant="primary"
        >
          Primary
        </Button>
        <Button
          className={css({ h: 'min-content' })}
          colors={{
            error: 'darkgreen',
            text: 'darkseagreen',
          }}
          danger
        >
          Danger
        </Button>
      </Flex>
      <Flex flexWrap="wrap" gap="12px" mb="16px">
        <Button
          className={css({ h: 'min-content' })}
          colors={{
            primary: 'steelblue',
          }}
        >
          Default
        </Button>
        <Button
          className={css({ h: 'min-content' })}
          colors={{
            primary: 'steelblue',
          }}
          variant="primary"
        >
          Primary
        </Button>
        <Button
          className={css({ h: 'min-content' })}
          colors={{
            error: 'steelblue',
          }}
          danger
        >
          Danger
        </Button>
      </Flex>
    </Box>
  )
}
