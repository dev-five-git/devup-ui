'use client'

import { css } from '@devup-ui/react'
import { forwardRef, type KeyboardEvent } from 'react'

import { clearInput } from '../../utils/dom'
import { Input, type InputProps } from '../Input'
import { GlassIcon } from '../Input/GlassIcon'

export interface SearchProps extends Omit<InputProps, 'type'> {
  /** Called with the current query when the user presses Enter */
  onSearch?: (value: string) => void
}

/** The browser's own clear button and decoration, which would sit beside the Input clear button */
const nativeSearchParts = css({
  selectors: {
    '&::-webkit-search-cancel-button, &::-webkit-search-decoration': {
      appearance: 'none',
    },
  },
})

export const Search = forwardRef<HTMLInputElement, SearchProps>(function Search(
  {
    icon = <GlassIcon />,
    onSearch,
    onKeyDown,
    onClear,
    allowClear = true,
    className,
    enterKeyHint = 'search',
    ...props
  },
  ref,
) {
  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    onKeyDown?.(e)
    // An Enter that confirms an IME composition (Korean, Japanese...) is not a search
    if (e.defaultPrevented || e.nativeEvent.isComposing) return
    const input = e.currentTarget
    if (e.key === 'Enter') {
      onSearch?.(input.value)
      return
    }
    if (
      e.key === 'Escape' &&
      allowClear &&
      input.value &&
      !input.readOnly &&
      !input.disabled
    ) {
      // Only an Escape that clears something is taken, so an empty search still lets it close a dialog
      e.preventDefault()
      clearInput(input)
      onClear?.()
    }
  }

  return (
    <Input
      ref={ref}
      allowClear={allowClear}
      className={`${nativeSearchParts} ${className || ''}`.trim()}
      enterKeyHint={enterKeyHint}
      icon={icon}
      onClear={onClear}
      onKeyDown={handleKeyDown}
      type="search"
      {...props}
    />
  )
})
