# Devup UI website

English stays at the original URLs; Korean pages live under /ko. Only the
Overview is translated initially. Missing Korean pages redirect to the English
URL, including without JavaScript. Unknown URLs still return 404.

The site uses next-intl for locale context and navigation. Because deployment is
a static export, it uses explicit App Router paths instead of middleware rewrites.
The shared [locale] catch-all renders translations and exports HTML redirects
for untranslated public routes. Translations sit beside their English source,
such as page.mdx and page.ko.mdx, and nothing else registers them: script.js
(run by dev, build and build:next) finds every page.ko.mdx and page.ko.tsx under
src/app/(detail) and writes the git-ignored src/i18n/pages.generated.ts. A
page's URL is its directory without route groups or [dynamic] segments, and it
renders inside the same layout.tsx files below (detail) as its English page.

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
5. Run node script.js (or restart bun dev) to pick up the new page and refresh
   search data. There is nothing to register.

For the team page, add src/app/(detail)/team/page.ko.mdx; it reuses the team
layout automatically. No Korean directory or layout wrapper is needed. Pages
without a Korean file keep the English redirect.

### Component references and TSX pages

For a component, add page.ko.tsx, index.ko.mdx and Api.ko.mdx beside its English
files in src/app/(detail)/components/[component]/<name>. Use the English renderer at
src/app/(detail)/components/[component]/page.tsx as the composition reference;
replace its computed MDX imports with imports from the translated files. Reuse
getDemos and MdxCard, and import shared code through @/ paths. Do not copy demos.
The page is served at /ko/components/<name> inside the components layout.

For home, showcase and components overview, add page.ko.tsx beside page.tsx
(home is src/app/(detail)/page.ko.tsx at /ko). Translate the component
overview's Description.mdx as Description.ko.mdx beside it. Reuse the interactive
components. Shared UI copy lives in src/i18n/messages/en.json and ko.json, one
namespace per component; add matching keys to both. src/i18n/request.ts merges
the English defaults with the Korean overrides per namespace, and SiteLayout
passes getMessages({locale}) to NextIntlClientProvider. Client components use
useTranslations; server components use getTranslations({locale}) from
next-intl/server with an explicit locale. Keep state identifiers and
component/API names unchanged.

The shared header, sidebars, table of contents and search use these catalogs. Search
indexes are built by script.js: Korean MDX replaces its English entry, and other
entries stay English. Use Link and usePathname from @/i18n/navigation for internal
navigation; use next/link for static assets such as /storybook/index.html.

## Local validation

From the repository root, run bun install and bun run build to build packages.
Then run bun dev and bun run build from apps/landing. Verify the English and
Korean URLs on desktop/mobile, including links, tables, code and search.

From the repository root:

```sh
node apps/landing/script.js # writes src/i18n/pages.generated.ts for tsc
bun ./node_modules/@typescript/native/bin/tsc --noEmit -p apps/landing/tsconfig.json
bun run lint
bunx playwright test e2e/landing-i18n.spec.ts
```

CI verifies both vinext and Next.js/Turbopack static exports. Limit translations
to the assigned page or shared-UI scope.
