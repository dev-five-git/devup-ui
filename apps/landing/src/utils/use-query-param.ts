'use client'

import { useSearchParams } from 'next/navigation'

interface QueryRouter {
  replace(href: string): void
}

export function useQueryParam(name: string) {
  return useSearchParams().get(name)
}

export function replaceQuery(router: QueryRouter, href: string) {
  router.replace(href)
}
