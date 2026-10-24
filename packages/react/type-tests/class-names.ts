import { type ClassNamesContent, cx } from '../src/compat'

const nested = [['nested', 7, true], { selected: true }] as const

cx(nested, 1)

declare const content: ClassNamesContent

content.cx(nested, 1)
