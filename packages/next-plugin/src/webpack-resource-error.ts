export type WebpackResourceBoundary = {
  readonly filename: string
  readonly rulePosition: string
  readonly test: string
  readonly unknownFact: string
  readonly reason: string
}

export class WebpackResourceError extends Error {
  readonly name = 'WebpackResourceError'
  readonly rulePosition: string
  readonly unknownFact: string
  constructor(
    readonly configFile: string,
    readonly boundary: WebpackResourceBoundary,
  ) {
    super(
      `${configFile}:1:1: devup-ui cannot certify initial input for \`${boundary.filename}\`: ${boundary.rulePosition} (test ${boundary.test}); unknown ${boundary.unknownFact}: ${boundary.reason}; make the input-producing rule resource-only and put Devup immediately before the recognized MDX compiler, remove source-replacing pitches/factories, or defer this boundary to a separately supported native-input protocol`,
    )
    this.rulePosition = boundary.rulePosition
    this.unknownFact = boundary.unknownFact
  }
}
