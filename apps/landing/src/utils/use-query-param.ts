'use client'

import { useSyncExternalStore } from 'react'

interface QueryRouter {
  replace(href: string): void
}

const listeners = new Set<() => void>()
let clientSearch = ''
let initialized = false
let listeningForPopState = false

function notifyListeners() {
  for (const listener of listeners) listener()
}

function syncFromLocation() {
  clientSearch = window.location.search
  initialized = true
  notifyListeners()
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  if (!listeningForPopState) {
    window.addEventListener('popstate', syncFromLocation)
    listeningForPopState = true
  }

  return () => {
    listeners.delete(listener)
    if (listeners.size === 0 && listeningForPopState) {
      window.removeEventListener('popstate', syncFromLocation)
      listeningForPopState = false
    }
  }
}

function getClientSnapshot() {
  if (!initialized) {
    clientSearch = window.location.search
    initialized = true
  }
  return clientSearch
}

function getServerSnapshot() {
  return ''
}

export function useQueryParam(name: string) {
  const search = useSyncExternalStore(
    subscribe,
    getClientSnapshot,
    getServerSnapshot,
  )

  return new URLSearchParams(search).get(name)
}

export function replaceQuery(router: QueryRouter, href: string) {
  clientSearch = new URL(href, window.location.href).search
  initialized = true
  notifyListeners()
  router.replace(href)
}
