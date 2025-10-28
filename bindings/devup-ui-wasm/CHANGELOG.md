# @devup-ui/wasm

## 1.0.44

### Patch Changes

- [#449](https://github.com/dev-five-git/devup-ui/pull/449) [`b85123f`](https://github.com/dev-five-git/devup-ui/commit/b85123fce0c097649bd602566cb3601107c190f6) Thanks [@owjs3901](https://github.com/owjs3901)! - Support selector with params

## 1.0.43

### Patch Changes

- [#435](https://github.com/dev-five-git/devup-ui/pull/435) [`8e46b5e`](https://github.com/dev-five-git/devup-ui/commit/8e46b5eb35897ff33b27738f4623ca7bee2588fb) Thanks [@owjs3901](https://github.com/owjs3901)! - Optimize memory files

## 1.0.42

### Patch Changes

- [#426](https://github.com/dev-five-git/devup-ui/pull/426) [`3c65364`](https://github.com/dev-five-git/devup-ui/commit/3c65364125cea6e3582562b99a9b71291fc6f8c2) Thanks [@owjs3901](https://github.com/owjs3901)! - Support layer on imports of globalCss

## 1.0.41

### Patch Changes

- [#400](https://github.com/dev-five-git/devup-ui/pull/400) [`fee97f8`](https://github.com/dev-five-git/devup-ui/commit/fee97f8a3d79b9a4c62858deb8e1aea8c609e3a2) Thanks [@owjs3901](https://github.com/owjs3901)! - Implement enum prop

## 1.0.40

### Patch Changes

- af157d4: Support void 0

## 1.0.39

### Patch Changes

- 8ae52ea: Impl Conditional as

## 1.0.38

### Patch Changes

- 338f974: Fix n, m logic to avoid adblock

## 1.0.37

### Patch Changes

- 88e3a5a: Add css banner

## 1.0.36

### Patch Changes

- f611579: Fix convert kebab issue in selectors

## 1.0.35

### Patch Changes

- d3b8502: Fix globalCss hot reload issue

## 1.0.34

### Patch Changes

- 629ad44: Optimized typography CSS values

  Merged base component CSS to avoid duplicate generation

  Introduced @layer to ensure style order consistency in CSS split mode

  Downgraded nm base to avoid issues with g-ad class (display: none when AdBlock is enabled)

  Fixed global CSS logic issue in CSS split mode

## 1.0.33

### Patch Changes

- 2247d57: Optimize keyframe name
- 9045f57: Feat split css

## 1.0.32

### Patch Changes

- 93f182c: Fix webkit-line-clamp issue

## 1.0.31

### Patch Changes

- 48a67da: Add webkit lineclamp to maintain value

## 1.0.30

### Patch Changes

- 5a854ba: Update wasm

## 1.0.29

### Patch Changes

- 2d51bf8: Optimize class gen logic

## 1.0.28

### Patch Changes

- e1416f8: Exclude elision in array

## 1.0.27

### Patch Changes

- 641c8ab: Update rust

## 1.0.26

### Patch Changes

- b6b2fed: Support auto wrapping for src in font faces
- b6b2fed: Support \_ selector and typing in globalCss

## 1.0.25

### Patch Changes

- c42d636: Impl font-faces to global

## 1.0.24

### Patch Changes

- c1c4f52: Optimize aspect-ratio by gcd

## 1.0.23

### Patch Changes

- 2116e88: Fix global css rerender issue without components
- 2116e88: Support css comment syntax
- 2116e88: Optimize css selector
- 2116e88: Optimize creating css

## 1.0.22

### Patch Changes

- 18bc846: Support \_selector in selectors prop
- 18bc846: Optimize selectors
- 18bc846: Optimize 0deg

## 1.0.21

### Patch Changes

- 39895fc: Optimize many func and props

## 1.0.20

### Patch Changes

- 80360c0: Fix multi imports issue
- 00c39d0: Split property
- c17424a: Optimize rgba func
- 1dd32af: Convert light-dark and optimize theme

## 1.0.19

### Patch Changes

- c352113: Optimize zero minus

## 1.0.18

### Patch Changes

- 469edf8: Fix optimize value

## 1.0.17

### Patch Changes

- 43729f0: Add default breakpoint

## 1.0.16

### Patch Changes

- f6e14e1: Fix media query issue

## 1.0.15

### Patch Changes

- c9593ce: css util support media query
  support devup json interface with wrong char
  optimize zero value

## 1.0.14

### Patch Changes

- 132e8dd: Implement globalCss, Fix ?? operator issue
- 946dbc2: Implement keyframe

## 1.0.13

### Patch Changes

- 2e1c60d: Refactor className logic

## 1.0.12

### Patch Changes

- e73fe8d: Fix className

## 1.0.11

### Patch Changes

- c27d60b: Support source map
- 8885c6c: Refactor classname
- 2ec5918: Fix or issue

## 1.0.10

### Patch Changes

- f6d3f0a: Support nested selector
- 373fb4e: Fix className issue (multiple selectors)

## 1.0.9

### Patch Changes

- 43dcfa5: Fix classname issue

## 1.0.8

### Patch Changes

- 940cd56: Feat auto insertion className, style props

## 1.0.7

### Patch Changes

- 1240507: Fix classname order issue

## 1.0.6

### Patch Changes

- 4e4bcf1: Apply optimize_value to dynamic value, remove semicolon on optimize_value
- 2dd81f5: Implement styleVars

## 1.0.5

### Patch Changes

- 5c3bb33: Maintain lineheight
- 0fdd0c4: Fix import issue, merged let if

## 1.0.4

### Patch Changes

- 9d45acb: Update wasm

## 1.0.3

### Patch Changes

- abfe5e5: Fix styleOrder issue

## 1.0.2

### Patch Changes

- bb2f49f: Add vendor properties (moz, webkit, ms)

## 1.0.1

### Patch Changes

- 94681ff: to phf
- 864c37b: Add info to package.json

## 1.0.0

### Major Changes

- 18b1bd7: Optimize Zero value

## 0.1.58

### Patch Changes

- b99448c: Rm content

## 0.1.57

### Patch Changes

- b937925: Add special props

## 0.1.56

### Patch Changes

- 9aba0a1: Apply css optimize

## 0.1.55

### Patch Changes

- 05036f7: Change breakpoint key

## 0.1.54

### Patch Changes

- 5f2ad1d: Add default theme

## 0.1.53

### Patch Changes

- 0bd2c42: Support TsAsExpression

## 0.1.52

### Patch Changes

- b9b4700: Support static member expression

## 0.1.51

### Patch Changes

- 0e1ab6e: Support for conditional selector

## 0.1.50

### Patch Changes

- 60ded6f: Optimize hex color with alpha

## 0.1.49

### Patch Changes

- c7ae821: Optimize value
- dd5ce79: Fix selector order in theme

## 0.1.48

### Patch Changes

- e644b46: Remove print

## 0.1.47

### Patch Changes

- 28c57b7: Fix dark selector issue

## 0.1.46

### Patch Changes

- f327278: Add maintain value

## 0.1.45

### Patch Changes

- 9c3f4cb: Add lineClamp

## 0.1.44

### Patch Changes

- 11c4257: Implement debug mode

## 0.1.43

### Patch Changes

- 4a1dd61: Fix selector issue
- 27c414d: Implement style order

## 0.1.42

### Patch Changes

- 6db4317: Add color schema

## 0.1.41

### Patch Changes

- 322e49b: Fix attribute selector issue

## 0.1.40

### Patch Changes

- d6e4605: Add call case

## 0.1.39

### Patch Changes

- b6997df: Add default value
- a0fc823: Fix conditional style issue

## 0.1.38

### Patch Changes

- bf2d907: Fix compound selector issue

## 0.1.37

### Patch Changes

- 553d754: Fix negative issue, literal issue

## 0.1.36

### Patch Changes

- 66ba223: Fix media issue

## 0.1.35

### Patch Changes

- bf47284: Support @media
- bf47284: Add special props

## 0.1.34

### Patch Changes

- b379bc6: Optimize to convert theme var

## 0.1.33

### Patch Changes

- 6f8e794: Support $ var with anywhere

## 0.1.32

### Patch Changes

- 2102fd7: Fix empty css call issue

## 0.1.31

### Patch Changes

- 0b434cd: Fix order issue

## 0.1.30

### Patch Changes

- 96cd069: Add flexGrow to Maintain prop

## 0.1.29

### Patch Changes

- c0d6030: Extract nested styles

## 0.1.28

### Patch Changes

- 525cd1e: Support variable for typography

## 0.1.27

### Patch Changes

- 67b53ac: Fix hmr issue

## 0.1.26

### Patch Changes

- eaf2737: Add aspectRatio to maintain value

## 0.1.25

### Patch Changes

- 570fa4c: Postfix convert to kebab

## 0.1.24

### Patch Changes

- 3aa26e6: Fix convert issue

## 0.1.23

### Patch Changes

- e072762: convert css

## 0.1.22

### Patch Changes

- 41aa78b: Add id to special prop

## 0.1.21

### Patch Changes

- 9d8e298: Add selector order

## 0.1.20

### Patch Changes

- 7f19bd5: Fix desctructing issue, Support for number in typo, Implement theme selector

## 0.1.19

### Patch Changes

- 00b14ca: Support destructing, Support to direct selection of object with theme, Support for template literal on as prop

## 0.1.18

### Patch Changes

- 5b53b1a: Fix transforming component from other package

## 0.1.17

### Patch Changes

- aba4b7e: Add type to special prop
- 873f596: Fix duplicate props issue

## 0.1.16

### Patch Changes

- 6f7c55d: Add group selector, Support conditional expression for typography

## 0.1.15

### Patch Changes

- 9fff520: Fix direct select issue

## 0.1.14

### Patch Changes

- 8943bad: Support double separator, Support backtick case

## 0.1.13

### Patch Changes

- 539e296: Fix walk issue, add flex props

## 0.1.12

### Patch Changes

- 0040445: Optimize str argument

## 0.1.11

### Patch Changes

- fd1f519: Add placeholder, maxLength, minLength to special properties
- 236882c: Add maintain properies
- b25c769: Fix ternary operator issue, css order issue, basic style issue

## 0.1.10

### Patch Changes

- 0de6892: Optimize media and Change sorting css var

## 0.1.9

### Patch Changes

- 1f2fb15: allow optional property of typography

## 0.1.8

### Patch Changes

- e9d655f: Add shorthand

## 0.1.7

### Patch Changes

- 7bde1f2: Add boxSizing prop

## 0.1.6

### Patch Changes

- 278d7f9: Add script

## 0.1.5

### Patch Changes

- b9b94c5: Add grid, minify css
- 242a6c2: Update package

## 0.1.4

### Patch Changes

- 2d7f28c: Change order to remove attr

## 0.1.3

### Patch Changes

- f7484c5: Support transpiled code

## 0.1.2

### Patch Changes

- 6100644: Add Image component

## 0.1.1

### Patch Changes

- f947f33: Fix gen classname logic with selector

## 0.1.0

### Minor Changes

- c0ff96f: Deploy
