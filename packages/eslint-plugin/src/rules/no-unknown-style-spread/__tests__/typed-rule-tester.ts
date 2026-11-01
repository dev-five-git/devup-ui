import { RuleTester } from '@typescript-eslint/rule-tester'
import { it } from 'bun:test'

export class TypedRuleTester extends RuleTester {
  static override get it(): typeof RuleTester.it {
    return (name: string, callback: () => void): void => {
      it(name, callback, 30000)
    }
  }
}
