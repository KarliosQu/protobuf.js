const { existsSync, readFileSync } = require('fs')
const { join } = require('path')

const { platform, arch } = process

let nativeBinding = null
let localFileExisted = false
let loadError = null

function isMusl() {
  // For Node 10
  if (!process.report || typeof process.report.getReport !== 'function') {
    try {
      const lddPath = require('child_process').execSync('which ldd').toString().trim()
      return readFileSync(lddPath, 'utf8').includes('musl')
    } catch (e) {
      return true
    }
  } else {
    const { glibcVersionRuntime } = process.report.getReport().header
    return !glibcVersionRuntime
  }
}

switch (platform) {
  case 'win32':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(join(__dirname, 'protobuf-core.win32-x64-msvc.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./protobuf-core.win32-x64-msvc.node')
          }
        } catch (e) {
          loadError = e
        }
        break
    }
    break
  case 'linux':
  case 'darwin':
      // Simplified for brevity in this environment, normally we list all
      try {
          nativeBinding = require('./protobuf-core.node'); 
      } catch (e) {
          // fallback
      }
      break;
}

if (!nativeBinding) {
  if (loadError) {
    throw loadError
  }
  throw new Error(`Failed to load native binding`)
}

const { Writer, Reader } = nativeBinding

module.exports.Writer = Writer
module.exports.Reader = Reader
