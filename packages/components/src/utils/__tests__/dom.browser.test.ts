import { describe, expect, it, mock } from 'bun:test'
import { createElement, createRef } from 'react'

import { elementRef, joinIds, mergeRefs } from '../dom'

describe('mergeRefs', () => {
  it('gives the node to callback refs and object refs, and takes it back', () => {
    const callback = mock()
    const object = createRef<HTMLDivElement>()
    const merged = mergeRefs<HTMLDivElement>(callback, object, undefined, null)
    const node = document.createElement('div')

    merged?.(node)
    expect(callback).toHaveBeenLastCalledWith(node)
    expect(object.current).toBe(node)

    merged?.(null)
    expect(callback).toHaveBeenLastCalledWith(null)
    expect(object.current).toBeNull()
  })
})

describe('mergeRefs without refs', () => {
  it('gives no ref when there is none to give the node to', () => {
    expect(mergeRefs(undefined, null)).toBeUndefined()
    expect(mergeRefs()).toBeUndefined()
  })
})

describe('joinIds', () => {
  it('joins the ids given and drops the missing ones', () => {
    expect(joinIds('a', undefined, false, 'b', null)).toBe('a b')
    expect(joinIds(undefined, false)).toBeUndefined()
  })
})

describe('elementRef', () => {
  it('reads the ref from the props in React 19 and from the element in React 18', () => {
    const ref = createRef<HTMLElement>()
    const element = createElement('div')
    const in19 = { ...element, props: { ref } } as typeof element
    const in18 = { ...element, ref } as typeof element

    expect(elementRef(in19, 19)).toBe(ref)
    expect(elementRef(in18, 18)).toBe(ref)
    expect(elementRef(element, 19)).toBeNull()
    expect(elementRef(element, 18)).toBeNull()
  })

  it('reads the version of React it runs on', () => {
    const ref = createRef<HTMLElement>()
    const element = createElement('div', { ref })

    expect(elementRef(element)).toBe(ref)
  })
})
