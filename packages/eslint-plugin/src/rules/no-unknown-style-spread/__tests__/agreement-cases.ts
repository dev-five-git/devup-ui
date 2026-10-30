type AgreementCase = {
  readonly name: string
  readonly body: string
  readonly outcome: 'pass-through' | 'composed' | 'refused'
  readonly reports: 0 | 1
}

export const agreementCases = [
  {
    name: 'local destructure default',
    body: 'export function render(props: { styles?: DevupProps }) { const { styles = knownImport } = props; return <Box {...styles} /> }',
    outcome: 'pass-through',
    reports: 1,
  },
  {
    name: 'parameter destructure default',
    body: 'export function render({ styles = knownImport }: { styles?: DevupProps }) { return <Box {...styles} /> }',
    outcome: 'pass-through',
    reports: 1,
  },
  {
    name: 'parameter default',
    body: 'export function render(styles: DevupProps = knownImport) { return <Box {...styles} /> }',
    outcome: 'pass-through',
    reports: 1,
  },
  {
    name: 'known-first mixed conditional',
    body: 'export function render(cond: boolean, rest: DevupProps) { return <Box {...(cond ? knownImport : rest)} /> }',
    outcome: 'refused',
    reports: 0,
  },
  {
    name: 'runtime-first mixed conditional',
    body: 'export function render(cond: boolean, rest: DevupProps) { return <Box {...(cond ? rest : knownImport)} /> }',
    outcome: 'refused',
    reports: 0,
  },
  {
    name: 'runtime-first nullish import fallback',
    body: 'export function render(rest: DevupProps | undefined) { return <Box {...(rest ?? knownImport)} /> }',
    outcome: 'pass-through',
    reports: 1,
  },
  {
    name: 'runtime-first or import fallback',
    body: 'export function render(rest: DevupProps | undefined) { return <Box {...(rest || knownImport)} /> }',
    outcome: 'pass-through',
    reports: 1,
  },
  {
    name: 'known import control',
    body: 'export function render() { return <Box {...knownImport} /> }',
    outcome: 'composed',
    reports: 0,
  },
  {
    name: 'runtime control',
    body: 'export function render(rest: DevupProps) { return <Box {...rest} /> }',
    outcome: 'pass-through',
    reports: 1,
  },
  {
    name: 'known-first nullish control',
    body: 'export function render(rest: DevupProps) { return <Box {...(knownImport ?? rest)} /> }',
    outcome: 'composed',
    reports: 0,
  },
  {
    name: 'known-first or control',
    body: 'export function render(rest: DevupProps) { return <Box {...(knownImport || rest)} /> }',
    outcome: 'composed',
    reports: 0,
  },
  {
    name: 'runtime-first nullish literal fallback',
    body: 'export function render(rest: DevupProps | undefined) { return <Box {...(rest ?? { bg: "red" })} /> }',
    outcome: 'pass-through',
    reports: 1,
  },
  {
    name: 'runtime-first or literal fallback',
    body: 'export function render(rest: DevupProps | undefined) { return <Box {...(rest || { bg: "red" })} /> }',
    outcome: 'pass-through',
    reports: 1,
  },
  {
    name: 'runtime-first or empty literal fallback',
    body: 'export function render(rest: DevupProps) { return <Box {...(rest || {})} /> }',
    outcome: 'pass-through',
    reports: 1,
  },
  {
    name: 'runtime-first or computed fallback',
    body: 'function make() { return { bg: "red" } } export function render(rest: DevupProps | undefined) { return <Box {...(rest || make())} /> }',
    outcome: 'pass-through',
    reports: 1,
  },
] as const satisfies readonly AgreementCase[]
