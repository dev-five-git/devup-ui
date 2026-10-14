/**
 * One argument of Emotion's `cx`: a class name, a falsy value that is skipped,
 * a class map (`{ name: condition }`) or an array of them.
 */
export type ClassNamesArg =
  | string
  | number
  | boolean
  | null
  | undefined
  | { [name: string]: unknown }
  | ClassNamesArg[]

/**
 * Emotion compatible class composition. The call is compiled away at build
 * time into the class string it names, so there is nothing to run.
 */
export function cx(..._classNames: ClassNamesArg[]): string {
  throw new Error('Cannot run on the runtime')
}

/**
 * Emotion compatible `merge`: the classes of a class string, composed like
 * `cx` composes its classes. Compiled away at build time like `cx`.
 */
export function merge(_className: string): string {
  throw new Error('Cannot run on the runtime')
}
