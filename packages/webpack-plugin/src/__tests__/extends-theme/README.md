# Packed Webpack extends-theme regression

After building WASM and all packages, run with current Node and Bun:

```sh
node --test packages/webpack-plugin/src/__tests__/extends-theme/extends-theme-regression.mjs
```

This is a production integration matrix, separate from mocked Bun unit tests. It packs the five public packages with lifecycle scripts disabled, resolves current registry versions once, installs independent consumers, and runs standalone Next-included Webpack (original `minimize: false`) and actual `next build --webpack`, each with per-file and single CSS. Two companion standalone builds directly import the public Webpack ESM entry, since the Next ESM config delegates to the CJS Webpack entry. No product rebuild is performed here.

Two more independent consumers each launch a fresh Node child using public `require` or direct `import`. The child initializes a real Webpack compiler and verifies three rule occurrences (JS/TS-pre, MDX-post, CSS-pre) using the two exported loader modules. It checks the default source domains are disjoint, compares every loader occurrence against the installed plugin manifest, and asserts the entry and all three occurrences resolve the same physical consumer WASM file and share its `getCss` function. Expected paths come from installed manifests, not build-tree paths. Each compiler is closed, including on assertion failure. The original eleven D fixture files remain unchanged.

Install the latest Playwright and its Chromium browser before running. `DEVUP_PLAYWRIGHT_ROOT` optionally selects an external tooling package root, `PLAYWRIGHT_BROWSERS_PATH` selects its browser cache, and `DEVUP_EXTENDS_RESULTS` selects the artifact parent. The runner checks Playwright against that run's registry snapshot. Consumer installs, lockfiles, package SHA256s, production output and observations are retained under the printed unique artifact directory for debugging.

The independent read-only `webpack-extends-theme` Linux CI job builds fresh WASM/packages, installs latest Playwright plus Chromium into a private `RUNNER_TEMP` package, and explicitly invokes this Node runner. These `.mjs` integration tests are not Bun-autodiscovered. Evidence is uploaded even on failure when available; the normal publish validation and changepack guard are unchanged.

The D fixture deliberately keeps `styles.mjs` separate, and keeps `devup.json` extending `base-theme.json` with primary `#17395b` and gutter `11px`. Tests parse declarations from HTTP-fetched **HTML-linked** stylesheets and check computed theme color/padding plus imported-style control `rgb(19, 87, 155)`/`9px`. A generated base file alone, variable references, mocked `getCss`, or injected CSS cannot satisfy this contract. Each browser/server is closed before the next cell; run this matrix under the caller's foreground process-tree memory/deadline guard.
