import assert from 'node:assert/strict'

export async function checkStyles(page, url) {
  const edited = process.env.RELEASE_REVISION === 'edited'
  const errors = []
  page.on('pageerror', (error) => errors.push(error.message))
  const response = await page.goto(url)
  assert.equal(response.status(), 200)
  await page.getByTestId('static').waitFor()
  const style = (id, property) =>
    page
      .getByTestId(id)
      .evaluate(
        (element, propertyName) =>
          getComputedStyle(element).getPropertyValue(propertyName),
        property,
      )
  assert.equal(await style('layout', 'display'), 'flex')
  assert.equal(await style('layout', 'flex-direction'), 'column')
  assert.equal(await style('layout', 'gap'), '12px')
  assert.equal(
    await style('static', 'background-color'),
    edited ? 'rgb(172, 104, 36)' : 'rgb(36, 104, 172)',
  )
  assert.equal(await style('static', 'padding-top'), '16px')
  assert.equal(await style('static', 'border-radius'), edited ? '0px' : '7px')
  assert.equal(
    await style('theme', 'color'),
    edited ? 'rgb(91, 57, 23)' : 'rgb(23, 57, 91)',
  )
  assert.equal(await style('theme', 'padding-left'), '11px')
  assert.equal(await style('imported', 'color'), 'rgb(19, 87, 155)')
  assert.equal(await style('imported', 'padding-top'), edited ? '8px' : '9px')
  await page.setViewportSize({ width: 320, height: 720 })
  assert.equal(await style('responsive', 'width'), '100px')
  await page.setViewportSize({ width: 1440, height: 900 })
  assert.equal(await style('responsive', 'width'), '200px')
  assert.equal(await style('hover', 'background-color'), 'rgb(17, 34, 51)')
  await page.getByTestId('hover').hover()
  assert.equal(await style('hover', 'background-color'), 'rgb(171, 205, 239)')
  assert.equal(await style('dynamic', 'background-color'), 'rgb(18, 52, 86)')
  await page.getByRole('button', { name: 'Change value' }).click()
  await page.waitForFunction(() => {
    const element = document.querySelector('[data-testid="dynamic"]')
    return (
      element &&
      getComputedStyle(element).backgroundColor === 'rgb(101, 67, 33)'
    )
  })
  assert.deepEqual(errors, [])
}
