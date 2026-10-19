import { existsSync, readFileSync, realpathSync } from 'node:fs'
import { cpus, freemem, totalmem } from 'node:os'
import { join, resolve } from 'node:path'

export function snapshot(ps) {
  return {
    at: new Date().toISOString(),
    freeBytes: freemem(),
    totalBytes: totalmem(),
    cpuTimes: cpus(),
    windows: JSON.parse(
      ps(
        "$ErrorActionPreference='Stop'; @{ cpu=@(Get-CimInstance Win32_Processor | Select-Object Name,LoadPercentage,NumberOfLogicalProcessors); system=Get-CimInstance Win32_PerfFormattedData_PerfOS_System | Select-Object ProcessorQueueLength,SystemUpTime; memory=Get-CimInstance Win32_OperatingSystem | Select-Object FreePhysicalMemory,TotalVisibleMemorySize } | ConvertTo-Json -Depth 5",
      ),
    ),
  }
}

export function validateCheckout({ root, cwd, app }, git) {
  if (
    process.env.RUST_MIN_STACK ||
    /stack[-_]?size|STACK:/i.test(
      `${process.env.RUSTFLAGS ?? ''} ${process.env.NODE_OPTIONS ?? ''}`,
    )
  )
    throw new Error('Default stack required')
  if (
    root.toLowerCase() !== resolve(process.cwd()).toLowerCase() ||
    realpathSync(root).toLowerCase() !== root.toLowerCase() ||
    realpathSync(cwd).toLowerCase() !== cwd.toLowerCase()
  )
    throw new Error(
      'Run from the selected checkout root without linked app paths',
    )
  for (const [file, name] of [
    ['package.json', 'devup-ui'],
    ['bindings/devup-ui-wasm/package.json', '@devup-ui/wasm'],
    ['packages/vite-plugin/package.json', '@devup-ui/vite-plugin'],
    [
      `apps/${app}/package.json`,
      app === 'landing' ? 'landing' : 'vite-example',
    ],
  ]) {
    if (JSON.parse(readFileSync(join(root, file), 'utf8')).name !== name)
      throw new Error(`Unexpected checkout package: ${file}`)
  }
  if (
    git(['rev-parse', '--show-toplevel'])
      .trim()
      .replaceAll('\\', '/')
      .toLowerCase() !== root.replaceAll('\\', '/').toLowerCase()
  )
    throw new Error('Selected directory must be the checkout root')
}

export function resolveCli({ app, cwd, binding, root }, result) {
  const cliPackage = app === 'landing' ? 'vinext' : 'vite'
  const packageDir = join(cwd, 'node_modules', cliPackage)
  const cliMetadata = JSON.parse(
    readFileSync(join(packageDir, 'package.json'), 'utf8'),
  )
  const cli = join(
    packageDir,
    typeof cliMetadata.bin === 'string'
      ? cliMetadata.bin
      : cliMetadata.bin[cliPackage],
  )
  if (
    !existsSync(cli) ||
    !existsSync(binding) ||
    !existsSync(join(root, 'apps/vite-lib/dist/index.js'))
  )
    throw new Error(
      'Prebuilt CLI, binding and vite-lib are required; runner never installs/builds dependencies',
    )
  result.cli = {
    package: cliPackage,
    version: cliMetadata.version,
    filename: realpathSync(cli),
  }
  return cli
}
