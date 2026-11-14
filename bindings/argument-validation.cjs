'use strict'
module.exports = function createBoundary() {
  const extraction =
    'filename:s code:s _package:s css_dir:s single_css:b import_main_css_in_code:b import_main_css_in_css:b import_aliases:aliases'
  const descriptors = {
    codeExtract: extraction,
    codeExtractWithoutSourceMap: extraction,
    getCss: 'file_num:n import_main_css:b',
    getThemeInterface:
      'package_name:s color_interface_name:s typography_interface_name:s length_interface_name:s shadows_interface_name:s theme_interface_name:s',
    hasDevupUI: 'filename:s code:s _package:s',
    importCanonicalMap: 'map_object:canonical',
    importClassMap: 'sheet_object:class',
    importFileMap: 'sheet_object:filemap',
    importFileRoutes: 'map_object:routes',
    importSheet: 'sheet_object:sheet',
    registerShorthands: 'shorthands:shorthands',
    registerTheme: 'theme_object:theme',
    reserveClassSlots:
      'rows:rows _package:s single_css:b import_main_css_in_code:b _import_main_css_in_css:b import_aliases:aliases',
    seedFileMap: 'files:a',
    setAtomHoist: 'threshold:n',
    setDebug: 'debug:b',
    setModuleResolver: 'resolver:f',
    setPrefix: 'prefix:p',
  }
  const labels = {
    s: 'a string',
    b: 'a boolean',
    n: 'a number, null or undefined',
    p: 'a string, null or undefined',
    f: 'a function, null or undefined',
    a: 'an array of strings',
    aliases: 'an alias mapping',
    canonical: 'a canonical mapping',
    class: 'a nested class-index mapping',
    filemap: 'a file-index mapping',
    routes: 'a file-to-route mapping',
    sheet: 'a sheet state',
    shorthands: 'a shorthand mapping',
    theme: 'a theme',
    rows: 'an iterable of 3-string rows',
  }
  const object = (value) =>
    value !== null && (typeof value === 'object' || typeof value === 'function')
  const checks = {
    s: (value) => typeof value === 'string',
    b: (value) => typeof value === 'boolean',
    n: (value) => value == null || typeof value === 'number',
    p: (value) => value == null || typeof value === 'string',
    f: (value) => value == null || typeof value === 'function',
    a: Array.isArray,
  }
  let frame
  const fail = (slot) => {
    frame.slot = slot
    return (frame.token ??= {})
  }
  const shape = (value, pair) => (pair ? value != null : object(value))
  const argumentError = (name, [parameter, kind]) =>
    new TypeError(`${name}(): argument ${parameter} must be ${labels[kind]}`)
  return {
    fail,
    shape,
    begin(slot) {
      frame.slot = slot
      frame.decoding = true
    },
    end() {
      frame.decoding = false
      frame.caught = null
    },
    caught(value) {
      if (frame?.decoding && (frame.token === null || value !== frame.token))
        (frame.caught ??= new Set()).add(value)
    },
    error(value) {
      const caught = frame.caught?.has(value)
      frame.decoding = false
      frame.caught = null
      return caught ? new Error(String(value)) : fail(frame.slot)
    },
    guard(value, pair = false) {
      // Typed map keys/sequence leaves reject this, retaining normal Rust cleanup.
      if (frame?.decoding && !shape(value, pair)) return pair ? [] : {}
      return value
    },
    wrap(api) {
      for (const [name, description] of Object.entries(descriptors)) {
        const slots = description.split(' ').map((entry) => entry.split(':'))
        const isExtraction = description === extraction
        const target = api[name]
        const apply = function (target, receiver, args) {
          const parent = frame
          frame = { slot: -1, token: null, caught: null, decoding: false }
          try {
            if (!(
              isExtraction &&
              typeof args[0] === 'string' &&
              typeof args[1] === 'string' &&
              typeof args[2] === 'string' &&
              typeof args[3] === 'string' &&
              typeof args[4] === 'boolean' &&
              typeof args[5] === 'boolean' &&
              typeof args[6] === 'boolean'
            )) {
              for (let slot = 0; slot < slots.length; slot++) {
                const check = checks[slots[slot][1]]
                if (check && !check(args[slot])) throw fail(slot)
              }
            }
            if (name === 'seedFileMap') {
              const files = args[0]
              void (files.length * 4)
              const copied = []
              for (let i = 0; i < files.length; i++) {
                const value = files[i]
                if (typeof value !== 'string') throw fail(0)
                copied.push(value)
              }
              const length = +files.length >>> 0
              if (length > copied.length) throw fail(0)
              copied.length = length
              args[0] = copied
            }
            return Reflect.apply(target, receiver, args)
          } catch (error) {
            if (frame.token !== null && error === frame.token) {
              throw argumentError(name, slots[frame.slot])
            }
            throw error
          } finally {
            frame = parent
          }
        }
        if (description === extraction) {
          const wrapped = function (...args) {
            const constructor = new.target === wrapped ? target : new.target
            if (constructor) return Reflect.construct(target, args, constructor)
            return apply(target, this, args)
          }
          Object.defineProperties(wrapped, {
            name: { value: target.name },
            length: { value: target.length },
            prototype: { value: target.prototype },
          })
          api[name] = wrapped
        } else api[name] = new Proxy(target, { apply })
      }
      return api
    },
  }
}
