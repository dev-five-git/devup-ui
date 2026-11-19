// What a build plugin writes to <distDir>/theme.d.ts for a devup.json, formatted
import '@devup-ui/react'
declare module '@devup-ui/react' {
  interface CustomColors {
    $primary: null
    [`$gray.100`]: null
  }
  interface DevupThemeTypography {
    heading: null
  }
  interface CustomLength {
    $containerX: null
  }
  interface CustomShadows {
    $card: null
  }
  interface DevupTheme {
    dark: null
    light: null
  }
}
