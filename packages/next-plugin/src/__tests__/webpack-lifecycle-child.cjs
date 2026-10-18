const { spawn } = require('node:child_process')

// Fault-only descendants deliberately ignore graceful shutdown.
process.on('SIGTERM', () => {})
if (process.argv[2] !== 'leaf')
  spawn(process.execPath, [__filename, 'leaf'], { stdio: 'inherit' })
console.info(`W65-FAULT-PID ${process.pid}`)
setInterval(() => {}, 1000)
