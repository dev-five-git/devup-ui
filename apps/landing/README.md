# Devup UI website

English stays at the original URLs; Korean pages live under /ko. Only the
Overview is translated initially. Missing Korean pages redirect to the English
URL, including without JavaScript. Unknown URLs still return 404.

The site uses next-intl for locale context and navigation. Because deployment is
a static export, it uses explicit App Router paths instead of middleware rewrites.
The shared [locale] catch-all renders registered translations and exports HTML
redirects for untranslated public routes. Translations sit beside their English
source, such as page.mdx and page.ko.mdx. A small map in src/i18n/pages.ts connects
each translated module to its public URL and shared layout.

## Contributing a Korean translation

Choose an issue labeled **good first issue** and **documentation**, comment to
claim it, and submit one page per PR. See the
[Korean translation issues](https://github.com/dev-five-git/devup-ui/issues?q=is%3Aissue+is%3Aopen+%22%5Bko%5D%22+label%3A%22good+first+issue%22).
Wait for [the infrastructure PR](https://github.com/dev-five-git/devup-ui/pull/881)
to merge before opening a translation PR.

### Documentation and MDX pages

1. Copy src/app/(detail)/docs/<path>/page.mdx to page.ko.mdx in the same directory.
   The existing Korean Overview is the example.
2. Translate headings, prose, tables, explanatory comments, alt text and metadata.
   Preserve API/package names, code behavior, numbers, qualifications and external URLs.
3. Keep internal links root-relative, such as /docs/installation. The shared
   next-intl Link adds the locale; untranslated targets redirect to English.
4. Set the canonical URL to /ko/docs/<path> and language alternatives to the
   English and Korean URLs. Keep the corresponding English metadata in sync.
5. Register the English pathname in src/i18n/pages.ts, following Overview:

```ts
'/docs/installation': {
  load: () => import('@/app/(detail)/docs/installation/page.ko.mdx'),
  Layout: DocsLayout,
},
```

For the team page, add src/app/(detail)/team/page.ko.mdx and register /team with
the existing team layout. Reuse layouts directly; no Korean directory or layout
wrapper is needed. Unregistered pages retain the English redirect. Run node
script.js to refresh search data; there is no generated translation registry.

### Component references and TSX pages

For a component, add page.ko.tsx, index.ko.mdx and Api.ko.mdx beside its English
files in src/app/(detail)/components/[component]/<name>. Use the English renderer at
src/app/(detail)/components/[component]/page.tsx as the composition reference;
replace its computed MDX imports with imports from the translated files. Reuse
getDemos and MdxCard, and import shared code through @/ paths. Do not copy demos.
Register /components/<name> in src/i18n/pages.ts with its page.ko.tsx loader and
the existing components layout.

For home, showcase and components overview, add page.ko.tsx beside page.tsx and
register its pathname in src/i18n/pages.ts (home uses /). Translate the component
overview's Description.mdx as Description.ko.mdx beside it. Reuse the interactive
components. Introduce matching keys in src/i18n/messages/en.json and ko.json,
and load catalogs in src/i18n/request.ts, merging English defaults with Korean
overrides per namespace. In SiteLayout, pass getMessages({locale}) to
NextIntlClientProvider. Client components use useTranslations; server components
use getTranslations({locale}) from next-intl/server with an explicit locale.
These catalogs are introduced by the first TSX/shared-UI translation contribution;
the current provider has no translated UI messages. Keep state identifiers and
component/API names unchanged.

Shared navigation, search labels and footer belong to their own issue. Search
indexes are built by script.js: Korean MDX replaces its English entry, and other
entries stay English. Use Link and usePathname from @/i18n/navigation for internal
navigation; use next/link for static assets such as /storybook/index.html.

## Local validation

From the repository root, run bun install and bun run build to build packages.
Then run bun dev and bun run build from apps/landing. Verify the English and
Korean URLs on desktop/mobile, including links, tables, code and search.

From the repository root:

```sh
bun ./node_modules/@typescript/native/bin/tsc --noEmit -p apps/landing/tsconfig.json
bun run lint
bunx playwright test e2e/landing-i18n.spec.ts
```

CI verifies both vinext and Next.js/Turbopack static exports. Limit translations
to the assigned page or shared-UI scope.
