# @DEVUP-UI/REACT - PUBLIC API

Core React package. Exports components, utilities, and types for zero-runtime CSS-in-JS.

## EXPORTS

### Components
`Box`, `Flex`, `Grid`, `VStack`, `Center`, `Text`, `Image`, `Input`, `Button`, `ThemeScript`

### Utilities
`styled()`, `css()`, `globalCss()`, `keyframes()`, `setTheme()`, `getTheme()`, `initTheme()`

### Hooks
`useTheme()`

### Types
`DevupProps`, `DevupTheme`, `DevupThemeTypography`, `DevupThemeTypographyKeys`

## STRUCTURE

```
src/
├── components/         # React components (compile-time only)
│   └── Box.tsx         # Base polymorphic component
├── utils/              # Styling utilities
│   ├── styled.ts       # styled-components API
│   ├── css.ts          # css() utility
│   └── global-css.ts   # globalCss()
├── hooks/
│   └── use-theme.ts    # useTheme hook
└── types/
    └── props/          # CSS property types (12 files)
        ├── background.ts
        ├── border.ts
        ├── selector/   # _hover, _focus, etc.
        └── ...
```

## WHERE TO LOOK

| Task | File | Notes |
|------|------|-------|
| Add prop shorthand | `types/props/*.ts` | Add to relevant file |
| Add selector | `types/props/selector/` | `_newSelector` pattern |
| Modify component | `components/*.tsx` | All throw at runtime |
| Add utility | `utils/` | Follow existing patterns |
| Theme types | `types/theme.ts` | Extends via module aug |

## CONVENTIONS

### Prop Naming
- Background: `bg`, `bgImage`, `bgColor` (NOT `background`)
- Spacing: `p`, `m`, `px`, `py`, `mt`, `mb` (NOT `padding`, `margin`)
- Size: `w`, `h`, `minW`, `maxH` (NOT `width`, `height`)

### Responsive Values
```tsx
<Box p={[1, 2, 3]} />        // [mobile, tablet, desktop]
<Box _hover={{ bg: ['red', 'blue'] }} />  // Responsive in selectors
```

### Selectors
- `_hover`, `_focus`, `_active` - pseudo classes
- `_before`, `_after` - pseudo elements
- `_themeDark`, `_themeLight` - theme-specific
- `_groupHover` - parent group selectors

### Theme Tokens
```tsx
<Text color="$primary" />     // Color token
<Text typography="$heading" /> // Typography preset
```

## ANTI-PATTERNS

- **NEVER** use runtime styling logic in components
- **NEVER** add client-side state for styling
- **NEVER** use `as any` in type definitions
- **AVOID** adding new dependencies

## TESTING

```bash
bun test                      # All tests
bun test --watch              # Watch mode
```

Uses `bun-test-env-dom` for DOM testing. 100% coverage required.
