export class Output {
  private constructor()
  free(): void
  [Symbol.dispose](): void
  readonly code: string
  readonly cssFile: string | undefined
  readonly css: string | undefined
  /** Each access returns an independent array; mutating it does not change native state. */
  readonly dependencies: string[]
  readonly map: string | undefined
  readonly updatedBaseStyle: boolean
}

// Positional signatures intentionally match the existing 34-name compiler ABI.
export function codeExtract(
  filename: string,
  code: string,
  packageName: string,
  cssDir: string,
  singleCss: boolean,
  importMainCssInCode: boolean,
  importMainCssInCss: boolean,
  importAliases: unknown,
): Output
export function codeExtractWithoutSourceMap(
  filename: string,
  code: string,
  packageName: string,
  cssDir: string,
  singleCss: boolean,
  importMainCssInCode: boolean,
  importMainCssInCss: boolean,
  importAliases: unknown,
): Output
/** Create an engine session with a clean state; select it before each call. */
export function createSession(): number
/** Drop a session and its state; dropping the live one makes session 0 live. */
export function dropSession(id: number): boolean
export function exportCanonicalMap(): string
export function exportClassMap(): string
export function exportFileMap(): string
export function exportSheet(): string
export function getCss(
  fileNum: number | null | undefined,
  importMainCss: boolean,
): string
export function getDefaultTheme(): string | undefined
export function getPrefix(): string | undefined
export function getThemeInterface(
  packageName: string,
  color: string,
  typography: string,
  length: string,
  shadows: string,
  theme: string,
): string
export function getUnreservedClassSlotCount(): number
export function hasDevupUI(
  filename: string,
  code: string,
  packageName: string,
  importAliases: unknown,
): boolean
export function importCanonicalMap(value: unknown): void
export function importClassMap(value: unknown): void
export function importFileMap(value: unknown): void
export function importFileRoutes(value: unknown): void
export function importSheet(value: unknown): void
export function isDebug(): boolean
export function registerShorthands(value: unknown): void
export function registerTheme(value: unknown): void
/** Forget a deleted source file's own styles; whether the sheet changed. */
export function removeFile(filename: string): boolean
export function reserveClassSlots(
  rows: unknown,
  packageName: string,
  singleCss: boolean,
  importMainCssInCode: boolean,
  importMainCssInCss: boolean,
  importAliases: unknown,
): void
export function resetBuildState(): void
export function resetStateForTesting(): void
export function seedFileMap(files: string[]): void
/** Make a session live: later calls work on its state until another is selected. */
export function selectSession(id: number): void
export function setAtomHoist(threshold?: number | null): void
export function setDebug(debug: boolean): void
export function setModuleResolver(
  resolver?:
    | ((
        specifier: string,
        importer: string,
      ) => { readonly path: string; readonly code: string } | null | undefined)
    | null,
): void
export function setPrefix(prefix?: string | null): void
export function setStylexStyleResolution(
  value?:
    | 'property-specificity'
    | 'application-order'
    | 'legacy-expand-shorthands'
    | null,
): void
