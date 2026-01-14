import * as fs from 'node:fs'

import * as wasm from '@devup-ui/wasm'
import {
  afterAll,
  afterEach,
  beforeAll,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import devupUICssLoader, { resetInit } from '../css-loader'

let getCssSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let importSheetSpy: ReturnType<typeof spyOn>
let importClassMapSpy: ReturnType<typeof spyOn>
let importFileMapSpy: ReturnType<typeof spyOn>
let existsSyncSpy: ReturnType<typeof spyOn>
let readFileSyncSpy: ReturnType<typeof spyOn>

const defaultOptions = {
  watch: false,
  sheetFile: 'sheet.json',
  classMapFile: 'classMap.json',
  fileMapFile: 'fileMap.json',
  themeFile: 'devup.json',
  theme: {},
  defaultSheet: {},
  defaultClassMap: {},
  defaultFileMap: {},
}

beforeAll(() => {
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('get css')
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  importSheetSpy = spyOn(wasm, 'importSheet').mockReturnValue(undefined)
  importClassMapSpy = spyOn(wasm, 'importClassMap').mockReturnValue(undefined)
  importFileMapSpy = spyOn(wasm, 'importFileMap').mockReturnValue(undefined)
  existsSyncSpy = spyOn(fs, 'existsSync').mockReturnValue(false)
  readFileSyncSpy = spyOn(fs, 'readFileSync').mockReturnValue('{}')
})

afterEach(() => {
  resetInit()
  getCssSpy.mockClear()
  registerThemeSpy.mockClear()
  importSheetSpy.mockClear()
  importClassMapSpy.mockClear()
  importFileMapSpy.mockClear()
  existsSyncSpy.mockClear()
  readFileSyncSpy.mockClear()
})

afterAll(() => {
  getCssSpy.mockRestore()
  registerThemeSpy.mockRestore()
  importSheetSpy.mockRestore()
  importClassMapSpy.mockRestore()
  importFileMapSpy.mockRestore()
  existsSyncSpy.mockRestore()
  readFileSyncSpy.mockRestore()
})

describe('devupUICssLoader', () => {
  it('should return css on no watch', () => {
    const callback = mock()
    const addContextDependency = mock()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      resourcePath: 'devup-ui.css',
      getOptions: () => ({ ...defaultOptions, watch: false }),
    } as any)(Buffer.from('data'), '')
    expect(callback).toHaveBeenCalledWith(
      null,
      Buffer.from('data'),
      '',
      undefined,
    )
    // Should initialize on first call
    expect(importFileMapSpy).toHaveBeenCalledTimes(1)
    expect(importClassMapSpy).toHaveBeenCalledTimes(1)
    expect(importSheetSpy).toHaveBeenCalledTimes(1)
    expect(registerThemeSpy).toHaveBeenCalledTimes(1)
  })

  it('should return _compiler hit css on watch', () => {
    const callback = mock()
    const addContextDependency = mock()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ ...defaultOptions, watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')
    expect(callback).toHaveBeenCalledTimes(1)
    expect(getCssSpy).toHaveBeenCalledTimes(1)
    getCssSpy.mockClear()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ ...defaultOptions, watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    expect(getCssSpy).toHaveBeenCalledTimes(1)

    getCssSpy.mockClear()

    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ ...defaultOptions, watch: true }),
      resourcePath: 'devup-ui-10.css',
    } as any)(Buffer.from(''), '')

    expect(getCssSpy).toHaveBeenCalledTimes(1)
  })

  it('should read files from disk in watch mode when files exist', () => {
    existsSyncSpy.mockReturnValue(true)
    readFileSyncSpy.mockReturnValue(JSON.stringify({ theme: { color: 'red' } }))

    const callback = mock()
    const addContextDependency = mock()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ ...defaultOptions, watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    // Should read files from disk
    expect(existsSyncSpy).toHaveBeenCalledTimes(4)
    expect(readFileSyncSpy).toHaveBeenCalledTimes(4)
    expect(importSheetSpy).toHaveBeenCalledTimes(1)
    expect(importClassMapSpy).toHaveBeenCalledTimes(1)
    expect(importFileMapSpy).toHaveBeenCalledTimes(1)
    expect(registerThemeSpy).toHaveBeenCalledWith({ color: 'red' })
  })

  it('should handle missing theme in devup.json', () => {
    existsSyncSpy.mockReturnValue(true)
    readFileSyncSpy.mockReturnValue(JSON.stringify({}))

    const callback = mock()
    const addContextDependency = mock()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ ...defaultOptions, watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    // Should call registerTheme with empty object when theme is missing
    expect(registerThemeSpy).toHaveBeenCalledWith({})
  })
})
