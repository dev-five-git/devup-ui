function isIdentifierChar(char: string | undefined): boolean {
  return char !== undefined && /[$\w]/.test(char)
}

class StaticVanillaParser {
  private index = 0
  private valid = true

  constructor(
    private readonly source: string,
    private readonly packageName: string,
  ) {}

  parse(): string | undefined {
    if (!this.word('import') || !this.char('{')) return undefined
    const imported = this.identifier()
    if (imported !== 'style') return undefined
    let local = imported
    if (this.word('as')) {
      const alias = this.identifier()
      if (alias === undefined) return undefined
      local = alias
    }
    if (
      !this.char('}') ||
      !this.word('from') ||
      this.string() !== '@vanilla-extract/css'
    ) {
      return undefined
    }
    this.char(';')

    const declarations: string[] = []
    while (!this.done()) {
      if (!this.word('export') || !this.word('const')) return undefined
      const name = this.identifier()
      if (name === undefined || !this.char('=') || !this.word(local)) {
        return undefined
      }
      if (!this.char('(')) return undefined
      this.skipTrivia()
      const valueStart = this.index
      if (!this.staticValue()) return undefined
      const value = this.source.slice(valueStart, this.index)
      if (!this.char(')')) return undefined
      this.char(';')
      declarations.push(`export const ${name} = css(${value})`)
    }

    if (declarations.length === 0) return undefined
    return `import { css } from '${this.packageName}'\n${declarations.join('\n')}`
  }

  private staticValue(): boolean {
    this.skipTrivia()
    const char = this.source[this.index]
    if (char === '{') return this.object()
    if (char === '[') return this.array()
    if (char === '"' || char === "'") return this.string() !== undefined
    if (char === '+' || char === '-') this.index++
    const number = this.source
      .slice(this.index)
      .match(/^(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?/i)
    if (number) {
      this.index += number[0].length
      return true
    }
    return this.word('true') || this.word('false') || this.word('null')
  }

  private object(): boolean {
    if (!this.char('{')) return false
    if (this.char('}')) return true
    while (true) {
      this.skipTrivia()
      const char = this.source[this.index]
      const key =
        char === '"' || char === "'"
          ? this.string()
          : (this.identifier() ?? this.numberKey())
      if (key === undefined || !this.char(':') || !this.staticValue()) {
        return false
      }
      if (this.char('}')) return true
      if (!this.char(',')) return false
      if (this.char('}')) return true
    }
  }

  private array(): boolean {
    if (!this.char('[')) return false
    if (this.char(']')) return true
    while (true) {
      if (!this.staticValue()) return false
      if (this.char(']')) return true
      if (!this.char(',')) return false
      if (this.char(']')) return true
    }
  }

  private numberKey(): string | undefined {
    this.skipTrivia()
    const match = this.source.slice(this.index).match(/^\d+(?:\.\d+)?/)
    if (!match) return undefined
    this.index += match[0].length
    return match[0]
  }

  private identifier(): string | undefined {
    this.skipTrivia()
    const match = this.source.slice(this.index).match(/^[$A-Z_a-z][$\w]*/)
    if (!match) return undefined
    this.index += match[0].length
    return match[0]
  }

  private string(): string | undefined {
    this.skipTrivia()
    const quote = this.source[this.index]
    if (quote !== '"' && quote !== "'") return undefined
    const start = ++this.index
    while (this.index < this.source.length) {
      const char = this.source[this.index++]
      if (char === '\\') {
        this.index++
      } else if (char === quote) {
        return this.source.slice(start, this.index - 1)
      } else if (char === '\n' || char === '\r') {
        return undefined
      }
    }
    return undefined
  }

  private word(value: string): boolean {
    this.skipTrivia()
    if (
      !this.source.startsWith(value, this.index) ||
      isIdentifierChar(this.source[this.index - 1]) ||
      isIdentifierChar(this.source[this.index + value.length])
    ) {
      return false
    }
    this.index += value.length
    return true
  }

  private char(value: string): boolean {
    this.skipTrivia()
    if (this.source[this.index] !== value) return false
    this.index++
    return true
  }

  private done(): boolean {
    this.skipTrivia()
    return this.valid && this.index === this.source.length
  }

  private skipTrivia(): void {
    while (this.index < this.source.length) {
      if (/\s/.test(this.source[this.index])) {
        this.index++
      } else if (this.source.startsWith('//', this.index)) {
        const newline = this.source.indexOf('\n', this.index + 2)
        this.index = newline === -1 ? this.source.length : newline + 1
      } else if (this.source.startsWith('/*', this.index)) {
        const end = this.source.indexOf('*/', this.index + 2)
        if (end === -1) {
          this.valid = false
          this.index = this.source.length
        } else {
          this.index = end + 2
        }
      } else {
        break
      }
    }
  }
}

/**
 * Convert the deliberately small, statically provable subset of vanilla
 * `style()` modules into Devup `css()` calls. Any executable expression or
 * additional statement returns undefined and keeps the existing Boa path.
 */
export function transformStaticVanillaExtract(
  filename: string,
  source: string,
  packageName: string,
): string | undefined {
  if (!/\.css\.(?:ts|js)$/.test(filename)) return undefined
  return new StaticVanillaParser(source, packageName).parse()
}
