'use client'
import { useEffect, useLayoutEffect } from 'react'

export const useSafeEffect: typeof useEffect =
  typeof window === 'undefined' ? useEffect : useLayoutEffect
