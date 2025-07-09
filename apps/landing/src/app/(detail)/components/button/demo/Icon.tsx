import { Button } from '@devup-ui/components'
import { Box, css } from '@devup-ui/react'

import IconDelete from '../IconDelete'

/**
 * **Icon**
 * Pass in an svg icon component into `icon` prop. If props like `stroke` and `fill` have `"currentColor"` value, the svg icon will follow the text color of the button.
 */
export function Icon() {
  return (
    <Box width="100%">
      <Box display="flex" flexWrap="wrap" gap="12px" marginBottom="16px">
        <Button
          className={css({ height: 'min-content' })}
          icon={
            <svg
              fill="none"
              height="24"
              viewBox="0 0 24 24"
              width="24"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                clipRule="evenodd"
                d="M16.2635 4.3205C15.8763 3.90288 15.2518 3.89202 14.8523 4.29596L6.92714 12.3101C6.77288 12.4661 6.66766 12.6701 6.6262 12.8938L6.19139 15.2388C6.04942 16.0044 6.67528 16.6795 7.38514 16.5264L9.56701 16.0557C9.74988 16.0163 9.91913 15.9232 10.0562 15.7868L16.6085 9.26287C16.6164 9.25496 16.6242 9.24687 16.6319 9.23862L18.0101 7.75198C18.4063 7.32464 18.4063 6.63179 18.0101 6.20445L16.2635 4.3205ZM15.1465 6.39842L15.5325 6.00805L16.4319 6.97821L16.058 7.38159L15.1465 6.39842ZM13.9617 7.59651L14.8868 8.59436L9.08091 14.3751L7.96212 14.6164L8.17961 13.4435L13.9617 7.59651ZM5.91304 18.0303C5.40878 18.0303 5 18.4712 5 19.0152C5 19.5591 5.40878 20 5.91304 20H18.087C18.5912 20 19 19.5591 19 19.0152C19 18.4712 18.5912 18.0303 18.087 18.0303H5.91304Z"
                fill="currentColor"
                fillRule="evenodd"
              />
            </svg>
          }
        >
          With icon
        </Button>

        <Button
          className={css({ height: 'min-content' })}
          icon={
            <svg
              fill="none"
              height="24"
              viewBox="0 0 25 24"
              width="25"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                d="M13.3333 7.83333C13.3333 7.3731 12.9602 7 12.5 7C12.0398 7 11.6667 7.3731 11.6667 7.83333V11.1667H8.33333C7.8731 11.1667 7.5 11.5398 7.5 12C7.5 12.4602 7.8731 12.8333 8.33333 12.8333H11.6667V16.1667C11.6667 16.6269 12.0398 17 12.5 17C12.9602 17 13.3333 16.6269 13.3333 16.1667V12.8333H16.6667C17.1269 12.8333 17.5 12.4602 17.5 12C17.5 11.5398 17.1269 11.1667 16.6667 11.1667H13.3333V7.83333Z"
                fill="currentColor"
              />
            </svg>
          }
          variant="primary"
        >
          Add
        </Button>
        <Button
          className={css({ height: 'min-content' })}
          danger
          icon={<IconDelete />}
        >
          Delete
        </Button>
      </Box>
    </Box>
  )
}
