import { setTheme } from '@devup-ui/react'
import { Decorator } from '@storybook/react'
import { useEffect } from 'react'
const preview: import('@storybook/react').Preview = {
  parameters: {
    layout: 'fullscreen',
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
  },
}

export default preview

export const globalTypes = {
  theme: {
    name: 'Theme',
    description: 'Global theme for components',
    defaultValue: 'light',
    toolbar: {
      // The icon for the toolbar item
      icon: 'circlehollow',
      // Array of options
      items: [
        { value: 'default', icon: 'circlehollow', title: 'light' },
        { value: 'dark', icon: 'circle', title: 'dark' },
      ],
      // Property that specifies if the name of the item will be displayed
      showName: true,
    },
  },
}

export const withHead: Decorator = (StoryFn) => {
  return (
    <>
      <StoryFn />
    </>
  )
}
export const withTheme: Decorator = (StoryFn, context) => {
  // Get values from story parameter first, else fallback to globals
  const theme = context.parameters.theme || context.globals.theme
  // eslint-disable-next-line react-hooks/rules-of-hooks
  useEffect(() => {
    setTheme(theme)
  }, [theme])
  return (
    <>
      <StoryFn />
      <style
        dangerouslySetInnerHTML={{
          __html: `:root[data-theme=default]{color-scheme: light;}:root[data-theme=dark]{color-scheme: dark;}`,
        }}
      ></style>
    </>
  )
}
export const decorators = [withHead, withTheme]
