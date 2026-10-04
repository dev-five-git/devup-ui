function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

export function preparedDiagnostics(
  filename: string,
  source: string,
  errors: readonly unknown[],
): string {
  return errors
    .flatMap((error) => {
      if (!isRecord(error)) return `${filename}:1:1: ${String(error)}`
      const labels = Array.isArray(error.labels)
        ? error.labels.filter(isRecord)
        : []
      if (labels.length) {
        return labels.map((label) => {
          const offset = typeof label.start === 'number' ? label.start : 0
          const lines = source.slice(0, offset).split('\n')
          return `${filename}:${lines.length}:${(lines.at(-1)?.length ?? 0) + 1}: ${String(error.message)}${typeof label.message === 'string' ? `: ${label.message}` : ''}`
        })
      }
      return `${filename}:${typeof error.line === 'number' ? error.line : 1}:${typeof error.column === 'number' ? error.column : 1}: ${String(error.message)}`
    })
    .join('\n')
}
