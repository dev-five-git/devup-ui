'use client'
import { Box, Input } from '@devup-ui/react'
import { useState } from 'react'

interface ToggleProps {
  defaultValue?: boolean | null
  value?: boolean | null
  onChange?: (value: boolean) => void
  disabled?: boolean
  variant?: 'default' | 'switch'
  className?: string
  style?: React.CSSProperties
  classNames?: {
    toggle?: string
  }
  styles?: {
    toggle?: React.CSSProperties
  }
  colors?: {
    primary?: string
    bg?: string
    hoverBg?: string
    primaryHoverBg?: string
    disabledBg?: string
    switchHoverOutline?: string
    switchShadow?: string
  }
}

export function Toggle({
  defaultValue = null,
  value = null,
  onChange,
  disabled,
  className,
  style,
  variant = 'default',
  colors,
  classNames,
  styles,
}: ToggleProps) {
  const [innerValue, setInnerValue] = useState<boolean>(
    value ?? defaultValue ?? false,
  )

  const resultValue = value ?? innerValue

  function handleToggle(value: boolean) {
    onChange?.(!value)
    setInnerValue((prev) => !prev)
  }

  const isDefault = variant === 'default'

  return (
    <>
      <Box
        aria-disabled={disabled}
        bg={
          resultValue
            ? 'var(--primary)'
            : 'var(--bg, light-dark(#E4E4E4, #383838))'
        }
        borderRadius="500px"
        boxSizing="border-box"
        className={className}
        cursor="pointer"
        h={isDefault ? '28px' : '8px'}
        justifyContent={resultValue && 'flex-end'}
        onClick={() => !disabled && handleToggle(resultValue)}
        p={!isDefault && 1}
        position="relative"
        role="group"
        selectors={{
          '&[aria-disabled=true]': {
            cursor: 'not-allowed',
            bg: 'var(--disabledBg, light-dark(#D6D7DE, #373737))',
          },
          '&:hover:not([aria-disabled=true]):not(:disabled)': {
            bg: resultValue
              ? `var(--primaryHoverBg, light-dark(color-mix(in srgb, var(--primary) 100%, #000 15%), color-mix(in srgb, var(--primary) 100%, #FFF 15%)))`
              : 'var(--hoverBg, light-dark(#C3C2C8, #696A6F))',
          },
        }}
        style={style}
        styleVars={{
          primary: colors?.primary,
          bg: colors?.bg,
          primaryHoverBg: colors?.primaryHoverBg,
          hoverBg: colors?.hoverBg,
          disabledBg: colors?.disabledBg,
        }}
        test-id="toggle-wrapper"
        transition=".25s"
        w={isDefault ? '50px' : '40px'}
      >
        <Box
          _groupHover={
            !isDefault && {
              outline: '4px solid',
              outlineColor: `var(--switchHoverOutline, light-dark(color-mix(in srgb, var(--primary) 20%, transparent), color-mix(in srgb, var(--primary) 50%, transparent)))`,
            }
          }
          backgroundColor="#fff"
          borderRadius="100%"
          boxSize="20px"
          className={classNames?.toggle}
          filter={
            !isDefault &&
            `drop-shadow(0px 0px 3px var(--switchShadow, rgba(0, 0, 0, 0.10)));`
          }
          outline="4px"
          pos="absolute"
          style={styles?.toggle}
          styleVars={{
            primary: colors?.primary,
            primaryHoverBg: colors?.primaryHoverBg,
            switchShadow: colors?.switchShadow,
            switchHoverOutline: colors?.switchHoverOutline,
          }}
          top={!isDefault && '-6px'}
          transform={resultValue && 'translateX(calc(100% + 2px))'}
          transition={isDefault ? '.25s' : 'transform .25s'}
        />
      </Box>
      <Input type="hidden" value={String(resultValue)} />
    </>
  )
}
