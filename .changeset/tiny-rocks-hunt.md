---
"@devup-ui/rsbuild-plugin": patch
"@devup-ui/webpack-plugin": patch
"@devup-ui/wasm": patch
"@devup-ui/next-plugin": patch
"@devup-ui/vite-plugin": patch
---

Optimized typography CSS values

Merged base component CSS to avoid duplicate generation

Introduced @layer to ensure style order consistency in CSS split mode

Downgraded nm base to avoid issues with g-ad class (display: none when AdBlock is enabled)

Fixed global CSS logic issue in CSS split mode


