import { type ReactElement, type Ref, type RefCallback, version } from 'react'

/** One ref callback giving the node to every ref it is given, in React 18 and 19 alike; none when there is no ref to give it to */
export function mergeRefs<T>(
  ...refs: (Ref<T> | undefined)[]
): RefCallback<T> | undefined {
  const given = refs.filter((ref) => ref != null)
  if (given.length === 0) return undefined
  return (node) => {
    for (const ref of given) {
      if (typeof ref === 'function') ref(node)
      else ref.current = node
    }
  }
}

/** A space separated token list of the ids given, without the missing ones */
export function joinIds(
  ...ids: (string | false | null | undefined)[]
): string | undefined {
  const joined = ids.filter(Boolean).join(' ')
  return joined || undefined
}

/** The ref of an element, which React 19 keeps in its props and React 18 beside them */
export function elementRef(
  element: ReactElement,
  major = Number.parseInt(version, 10),
): Ref<HTMLElement> {
  const holder = major >= 19 ? element.props : element
  return (holder as { ref?: Ref<HTMLElement> }).ref ?? null
}
