import { describe, expect, it } from 'bun:test'

import {
  getLocale,
  localizeHref,
  stripLocale,
} from '../../apps/landing/src/i18n/config'
import { translateMessage } from '../../apps/landing/src/i18n/messages'
import { localizedMetadata } from '../../apps/landing/src/i18n/metadata'
import {
  englishRoutes,
  translatedRoutes,
} from '../../apps/landing/src/i18n/routes.generated'

describe('landing locale URLs', () => {
  it.each([
    ['/', '/ko'],
    ['/docs/overview', '/ko/docs/overview'],
    ['/components/button?demo=1#api', '/ko/components/button?demo=1#api'],
    ['/showcase', '/ko/showcase'],
    ['/team', '/ko/team'],
    ['/?search=1#top', '/ko/?search=1#top'],
  ])('localizes %s and removes the prefix again', (english, korean) => {
    expect(localizeHref(english, 'ko')).toBe(korean)
    expect(localizeHref(korean, 'en')).toBe(english)
    expect(localizeHref(korean, 'ko')).toBe(korean)
    expect(getLocale(korean)).toBe('ko')
    expect(getLocale(english)).toBe('en')
  })

  it.each([
    '#examples',
    '?search=1',
    'https://example.com/docs',
    '//example.com/docs',
    'mailto:hello@example.com',
    '/storybook/index.html',
    '/logo.svg',
    '/korean',
    '/docs-other',
    '/team-other',
  ])('keeps non-page link %s intact', (href) => {
    expect(localizeHref(href, 'ko')).toBe(href)
  })

  it('recognizes locale boundaries, queries and root paths', () => {
    expect(stripLocale('/ko/')).toBe('/')
    expect(stripLocale('/ko?search=1')).toBe('/?search=1')
    expect(localizeHref('/ko?search=1', 'en')).toBe('/?search=1')
    expect(getLocale('/ko#contents')).toBe('ko')
    expect(stripLocale('/korean/docs')).toBe('/korean/docs')
  })
})

it('keeps the public route registry unique and prefixes only translated URLs', () => {
  expect(new Set(englishRoutes).size).toBe(englishRoutes.length)
  expect(englishRoutes).toContain('/components/button')
  expect(englishRoutes).toContain('/docs/overview')
  expect(englishRoutes.every((route) => !route.startsWith('/ko'))).toBe(true)
  expect(translatedRoutes).toEqual(['/docs/overview'])
})

it('keeps untranslated UI strings in English', () => {
  expect(translateMessage('Docs', 'en')).toBe('Docs')
  expect(translateMessage('Docs', 'ko')).toBe('Docs')
})

it('advertises real translations and canonicalizes English fallbacks', () => {
  const source = { title: 'Overview', openGraph: { siteName: 'Devup UI' } }
  const translated = localizedMetadata(source, '/docs/overview', true)
  expect(translated.alternates).toEqual({
    canonical: '/ko/docs/overview',
    languages: {
      en: '/docs/overview',
      ko: '/ko/docs/overview',
      'x-default': '/docs/overview',
    },
  })
  expect(translated.openGraph).toMatchObject({
    siteName: 'Devup UI',
    url: '/ko/docs/overview',
    locale: 'ko_KR',
  })
  expect(localizedMetadata({}, '/docs/installation', false).alternates).toEqual(
    {
      canonical: '/docs/installation',
      languages: {
        en: '/docs/installation',
        'x-default': '/docs/installation',
      },
    },
  )
})
