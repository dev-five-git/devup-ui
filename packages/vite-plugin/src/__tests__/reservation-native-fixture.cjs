const {
  existsSync,
  mkdirSync,
  writeFileSync,
  rmSync,
  symlinkSync,
} = require('node:fs')
const { dirname, join } = require('node:path')

/** @param {string} root @param {string} path @param {string} code */
function put(root, path, code) {
  mkdirSync(dirname(join(root, path)), { recursive: true })
  writeFileSync(join(root, path), code)
}
/** @param {string} name @param {string} color */
function style(name, color) {
  return `import {Box} from '@devup-ui/react'; export const ${name}=(value:string)=><Box color="${color}" w={value} _hover={{bg:'${color}'}}/>; console.log(${name})`
}
/** @param {string} root @param {string} scenario @param {string} mode */
function fixture(root, scenario, mode) {
  if (scenario === 'history')
    for (const name of ['newA', 'newB'])
      rmSync(join(root, `src/${name}.tsx`), { force: true })
  const write = (/** @type {string} */ path, /** @type {string} */ code) =>
    put(root, path, code)
  write('devup.json', '{}')
  write(
    'package.json',
    JSON.stringify({ name: 'native-fixture', type: 'module' }),
  )
  write('physical/A.tsx', style('A', 'red'))
  write('physical/B.tsx', style('B', 'blue'))
  write(
    'index.html',
    '<script type="module" src="/physical/A.tsx"></script><script type="module" src="/physical/B.tsx"></script>',
  )
  write(
    'unused.tsx',
    "import {css} from '@devup-ui/react'; css({color:window.never})",
  )
  write(
    'src/unused-invalid.tsx',
    "import {css} from '@devup-ui/react'; css({color:window.never})",
  )
  write(
    'src/unused-valid.tsx',
    "import {css} from '@devup-ui/react'; css({color:'sentinel-unreachable'})",
  )
  for (const directory of ['out', 'df']) {
    write(`${directory}/stale.tsx`, style('StaleOutput', 'sentinel-stale'))
    write(
      `${directory}/quoted.js`,
      `const fake = "import './stale.tsx'"; console.log(fake)`,
    )
  }
  write(
    'node_modules/accepted/package.json',
    JSON.stringify({
      name: 'accepted',
      type: 'module',
      exports: './dist/index.js',
    }),
  )
  write(
    'node_modules/accepted/unused.js',
    "import {css} from '@devup-ui/react'; css({color:window.never})",
  )
  write(
    'node_modules/accepted/dist/index.js',
    "import {css} from '@devup-ui/react'; export const accepted=css({borderColor:'gold'})",
  )
  /** @type {import('vite').BuildOptions} */
  let build = {}
  let base = '/'
  let paths = ['/physical/A.tsx', '/physical/B.tsx']
  switch (scenario) {
    case 'hoist-lazy':
      paths = require('./reservation-native-lazy-fixture.cjs').lazyFixture(
        root,
        write,
      )
      break
    case 'hoist':
      if (!existsSync(join(root, 'node_modules/react')))
        symlinkSync(
          dirname(
            require.resolve('react/package.json', {
              paths: [join(__dirname, '../../../react')],
            }),
          ),
          join(root, 'node_modules/react'),
          'junction',
        )
      write(
        'shared/S.tsx',
        "import {styled} from '@devup-ui/react'; export const Shared=styled('div',{color:'orchid',p:7})",
      )
      for (const [name, opacity] of [
        ['A', '.25'],
        ['B', '.75'],
      ])
        write(
          `physical/${name}.tsx`,
          `import {Box} from '@devup-ui/react'; import {Shared} from '../shared/S'; export const ${name}=(value:string)=><><Shared/><Box opacity={${opacity}} w={value}/></>; console.log(${name})`,
        )
      paths.push('/shared/S.tsx')
      break
    case 'percent':
    case 'devfalse':
      write('physical/© A.tsx', style('A', 'red'))
      write(
        'index.html',
        '<script type="module" src="/physical/%C2%A9%20A.tsx"></script><script type="module" src="/physical/B.tsx"></script>',
      )
      paths = ['/physical/%C2%A9%20A.tsx', '/physical/B.tsx']
      break
    case 'base':
      base = '/nested/base/'
      write(
        'index.html',
        mode === 'serve'
          ? '<script type="module" src="%BASE_URL%physical/A.tsx"></script><script type="module" src="%BASE_URL%physical/B.tsx"></script>'
          : '<script type="module" src="/physical/A.tsx"></script><script type="module" src="/physical/B.tsx"></script>',
      )
      paths = ['/nested/base/physical/A.tsx', '/nested/base/physical/B.tsx']
      break
    case 'declared':
      write(
        'pages/declared.html',
        '<script type="module" src="../physical/A.tsx"></script><script type="module" src="../physical/B.tsx"></script>',
      )
      build = { rolldownOptions: { input: 'pages/declared.html' } }
      break
    case 'multiple':
    case 'mixed':
      write(
        'pages/a.html',
        '<script type="module" src="../physical/A.tsx"></script>',
      )
      write(
        'pages/b.html',
        '<script type="module" src="../physical/B.tsx"></script>',
      )
      write('root.js', 'import "./physical/B.tsx"')
      build = {
        rolldownOptions: {
          input:
            scenario === 'mixed'
              ? ['pages/a.html', 'root.js']
              : ['pages/a.html', 'pages/b.html'],
        },
      }
      break
    case 'inline':
      write(
        'index.html',
        '<script type="module">import "./physical/A.tsx"; import "./physical/B.tsx"</script>',
      )
      break
    case 'root':
    case 'lib':
    case 'ssr':
    case 'packages':
      write(
        'root.js',
        `import './physical/A.tsx'; import './physical/B.tsx'; ${scenario === 'packages' ? "import {accepted} from 'accepted'; import {style} from '@vanilla-extract/css'; console.log(accepted,style({outlineColor:'purple'}))" : ''}`,
      )
      build =
        scenario === 'lib'
          ? { lib: { entry: 'root.js', formats: ['es'] } }
          : scenario === 'ssr'
            ? { ssr: 'root.js' }
            : { rolldownOptions: { input: 'root.js' } }
      break
    case 'environments':
    case 'serve-ssr':
      write('server.tsx', style('Server', 'navy'))
      break
    case 'hmr':
    case 'retained':
    case 'linked':
    case 'history':
      write(
        'src/A.tsx',
        `import {Box} from '@devup-ui/react'; import {tone} from '${scenario === 'linked' ? 'accepted' : '../supplier.js'}'; ${['retained', 'linked'].includes(scenario) ? 'export const runtimeTone=()=>tone;' : ''} export const A=(value:string)=><Box color={tone} w={value}/>; console.log(A); import.meta.hot?.accept()`,
      )
      write('supplier.js', "export const tone='maroon'")
      write('index.html', '<script type="module" src="/src/A.tsx"></script>')
      paths = ['/src/A.tsx']
      if (scenario === 'linked') {
        write(
          'workspace/accepted/package.json',
          JSON.stringify({
            name: 'accepted',
            type: 'module',
            exports: './dist/index.js',
          }),
        )
        write(
          'workspace/accepted/dist/index.js',
          "import {css} from '@devup-ui/react'; export const tone='maroon'; export const compiled=css({borderColor:'gold'})",
        )
        write(
          'workspace/accepted/unused.js',
          "import {css} from '@devup-ui/react'; css({color:window.never})",
        )
        rmSync(join(root, 'node_modules/accepted'), {
          recursive: true,
          force: true,
        })
        symlinkSync(
          join(root, 'workspace/accepted'),
          join(root, 'node_modules/accepted'),
          'junction',
        )
      }
      break
    case 'error':
      write(
        'root.js',
        "import {css} from '@devup-ui/react'; console.log(css({color:window.reachable}))",
      )
      build = { rolldownOptions: { input: 'root.js' } }
      break
    case 'virtual':
      write('root.js', "import 'virtual:styled'")
      build = { rolldownOptions: { input: 'root.js' } }
      break
    case 'default':
    case 'stale':
      break
    default:
      throw new TypeError(`Unknown scenario ${scenario}`)
  }
  return { write, build, base, paths }
}
module.exports = { fixture, style, put }
