# Devup UI website

English is the source of truth and stays at the original URLs. Korean lives at
`/ko`, for example `/ko/docs/overview`. There is no browser-language redirect.
An untranslated Korean URL renders its English source, with an English `lang`
attribute on that content and a canonical URL pointing to the English page.
Unknown URLs still return 404. All routes work on a static host without a server
or middleware.

## Contributing a Korean translation

Pick one open issue labeled **good first issue** and **documentation**, comment
that you would like to work on it, and submit one page per PR. The current
translation is `src/content/ko/docs/overview/page.mdx`; use it as a reference.
Browse the [Korean translation issues](https://github.com/dev-five-git/devup-ui/issues?q=is%3Aissue+is%3Aopen+%22%5Bko%5D%22+label%3A%22good+first+issue%22).

### Documentation pages

1. Copy `src/app/(detail)/docs/<path>/page.mdx` to
   `src/content/ko/docs/<path>/page.mdx`.
2. Translate headings, prose, table labels, explanatory comments, image alt text,
   and metadata title/description. Keep APIs, package names, code behavior,
   numbers, benchmark qualifications, imports, and external URLs intact.
3. Use root-relative internal links such as `/docs/installation`. The shared MDX
   link component adds `/ko` while browsing Korean pages. Links to untranslated
   pages automatically display the English source.
4. Set the translation's canonical to `/ko/docs/<path>`. The route supplies
   language alternatives and keeps English fallback pages canonicalized to English.

No routing changes or manual translation registry entries are needed. The
`dev`, `build`, and `build:next` scripts discover translated files automatically.

### Component reference pages

Copy both English MDX files from
`src/app/(detail)/components/[component]/<name>/` into
`src/content/ko/components/<name>/`:

```text
src/content/ko/components/button/index.mdx
src/content/ko/components/button/Api.mdx
```

Translate their prose, headings, table descriptions and explanatory comments.
Preserve the MDX component overrides and API identifiers. Interactive examples
and their source code are shared automatically; do not copy the `demo` folder.
Each missing MDX file independently falls back to English. The Korean URL becomes
canonical once both files are translated.

### Shared UI and TSX pages

`src/i18n/messages/ko.json` is intentionally empty initially. Its keys are the
exact English source strings and its values are Korean translations. Missing
keys display English. Navigation labels already use this dictionary. Wrap
additional visible strings with `<Message>English source text</Message>` from
`@/i18n/Message`; use `translateMessage(text, locale)` for attributes. Keep
state values, filter identifiers and API names in English.

For TSX pages (home, showcase, components overview), keep the shared interactive
components, add dictionary entries and Message wrappers within the relevant
components, then add `src/content/ko/<route>/page.tsx` with translated metadata
that renders the shared page. For home the file is `src/content/ko/page.tsx`.
Use absolute `@/` imports if copying a file containing relative imports. The
team page follows the MDX convention at `src/content/ko/team/page.mdx`.

Generated `src/i18n/*.generated.ts` files contain explicit imports for both
bundlers; regenerate them rather than editing them by hand. Search uses Korean
content where available and retains English entries for untranslated pages.
The language switcher preserves the current page, query and fragment.

## Local validation

From the repository root:

```sh
bun install
bun run build
```

Then from `apps/landing`:

```sh
bun dev
# Open /docs/<path> and /ko/docs/<path> (or /ko/components/<name>).
# Check desktop/mobile, headings, tables, links and code blocks.
bun ../../node_modules/@typescript/native/bin/tsc --noEmit
bun run build
```

From the repository root, format and run the focused checks:

```sh
bun run lint:fix
bun run lint
bun test ./packages/__tests__/landing-i18n.test.ts
bunx playwright test e2e/landing-i18n.spec.ts
```

The production build verifies HTML and RSC artifacts for every English and Korean
route. CI also builds with Next.js/Turbopack and runs the route and browser tests.
Keep the English page unchanged when contributing a translation; limit shared
code changes to the Message wrappers required for the assigned TSX/UI scope.
