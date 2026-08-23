type DemoModule = {
  default: React.ComponentType
}

const demoModules = import.meta.glob<DemoModule>('../app/**/demo/*.tsx', {
  eager: true,
})

export async function getDemos(
  dir: string,
): Promise<[React.ComponentType, string][]> {
  const directoryMarker = `/${dir}/demo/`

  return Object.entries(demoModules)
    .filter(([path]) => path.includes(directoryMarker))
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([path, module]) => {
      const filename = path.slice(path.lastIndexOf('/') + 1)

      return [module.default, `${dir}/demo/${filename}`]
    })
}
