import { controls, guarded, independent } from './owner-export-inventory.js'

export function rewriteOwnerExports(source, runtime) {
  const counts = new Map()
  const known = new Set([...guarded, ...independent, ...controls])
  if (known.size !== guarded.length + independent.length + controls.length)
    throw new Error('duplicate owner export inventory')
  for (const assignment of source.matchAll(
    /^\s*(?:module\.)?exports(?:\b|\[).*$/gm,
  )) {
    if (!/^exports\.\w+ = \w+;$/.test(assignment[0].trim()))
      throw new Error('unrecognized generated export assignment')
  }
  const result = source.replace(
    /^exports\.(\w+) = (\w+);$/gm,
    (assignment, name, local) => {
      if (!known.has(name) || name !== local)
        throw new Error(`unknown generated export: ${name}`)
      counts.set(name, (counts.get(name) ?? 0) + 1)
      if (controls.includes(name)) return ''
      const opcode = guarded.indexOf(name)
      return opcode < 0
        ? assignment
        : `exports.${name} = __ownerRuntime.wrap(${local}, ${opcode}, '${name}');`
    },
  )
  for (const name of known)
    if (counts.get(name) !== 1)
      throw new Error(`generated export must occur once: ${name}`)
  const initialization = `const __ownerRuntime = (${runtime.toString()})({${controls.join(',')}});\n`
  const header = result.match(/^\/\* @ts-self-types=.*?\*\/\r?\n/)
  const guardedSource = header
    ? header[0] + initialization + result.slice(header[0].length)
    : initialization + result
  return guardedSource + '\nexports.ownership = __ownerRuntime.ownership;\n'
}
