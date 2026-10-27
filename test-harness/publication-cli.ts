import {
  acceptPublication,
  beginPublication,
  continuePublication,
  recordRoot,
  recordVite,
} from './publication'

const action = process.argv[2]
switch (action) {
  case 'begin':
  case 'standalone':
    beginPublication(process.cwd())
    break
  case 'continue':
    continuePublication(process.cwd())
    break
  case 'root':
    recordRoot(process.cwd())
    break
  case 'accept':
    acceptPublication(process.cwd())
    break
  case 'vite':
    recordVite(process.cwd())
    break
  default:
    throw new TypeError(`Unknown publication action: ${action}`)
}
