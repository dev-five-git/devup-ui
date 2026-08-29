const LINE_BREAK = '<br>'

export function splitCodeLines(code: string) {
  let offset = 0

  return code.split(LINE_BREAK).map((text) => {
    const line = {
      key: offset,
      startsNewLine: offset > 0,
      text,
    }

    offset += text.length + LINE_BREAK.length
    return line
  })
}
