import type { Metadata } from 'next'

import { siteMetadata } from '../constants/site-metadata'
import { localizeHref } from './config'

export function localizedMetadata(
  metadata: Metadata,
  path: string,
  translated: boolean,
): Metadata {
  return {
    ...siteMetadata,
    ...metadata,
    alternates: {
      canonical: translated ? localizeHref(path, 'ko') : path,
      languages: {
        en: path,
        ...(translated ? { ko: localizeHref(path, 'ko') } : {}),
        'x-default': path,
      },
    },
    openGraph: {
      ...siteMetadata.openGraph,
      ...metadata.openGraph,
      title: metadata.openGraph?.title ?? metadata.title ?? siteMetadata.title,
      description:
        metadata.openGraph?.description ??
        metadata.description ??
        siteMetadata.description,
      url: translated ? localizeHref(path, 'ko') : path,
      locale: translated ? 'ko_KR' : 'en_US',
    },
  }
}
