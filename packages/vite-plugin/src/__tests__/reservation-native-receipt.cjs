const assert = require('node:assert/strict')

/** @param {unknown} value @returns {Record<string, unknown>} */
function record(value) {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value))
  return Object.fromEntries(Object.entries(value))
}
/** @param {unknown} value */
function text(value) {
  assert.equal(typeof value, 'string')
  assert(typeof value === 'string')
  return value
}
/** @param {unknown} value */
function number(value) {
  assert(typeof value === 'number')
  return value
}
/** @param {unknown} value */
function boolean(value) {
  assert(typeof value === 'boolean')
  return value
}
/** @param {unknown} value @returns {readonly unknown[]} */
function array(value) {
  assert(Array.isArray(value))
  return value
}
/** @param {unknown} value */
function strings(value) {
  return array(value).map(text)
}
/** @param {unknown} value */
function dictionary(value) {
  return Object.fromEntries(
    Object.entries(record(value)).map(([key, val]) => [key, text(val)]),
  )
}
/** @param {unknown} value */
function extraction(value) {
  const row = record(value)
  return {
    id: text(row.id),
    input: text(row.input),
    code: text(row.code),
    css: row.css === undefined ? undefined : text(row.css),
    map: row.map === undefined ? undefined : text(row.map),
    cssFile: row.cssFile === undefined ? undefined : text(row.cssFile),
    dependencies: strings(row.dependencies),
    updatedBaseStyle: boolean(row.updatedBaseStyle),
    before: number(row.before),
    after: number(row.after),
  }
}
/** @param {unknown} value */
function reservation(value) {
  const row = record(value)
  return {
    rows: array(row.rows).map(strings),
    flags: array(row.flags),
    classMap: text(row.classMap),
    fileMap: text(row.fileMap),
  }
}
/** @param {string} raw */
function parseReceipt(raw) {
  const value = record(JSON.parse(raw))
  return {
    events: strings(value.events),
    reservations: array(value.reservations).map(reservation),
    extractions: array(value.extractions).map(extraction),
    classMap: text(value.classMap),
    fileMap: text(value.fileMap),
    css: text(value.css),
    count: number(value.count),
    assets: value.assets === undefined ? undefined : dictionary(value.assets),
    http: value.http === undefined ? undefined : dictionary(value.http),
  }
}
module.exports = { parseReceipt, record, text, array, strings }
