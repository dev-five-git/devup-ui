import { expect, test } from '@playwright/test'

test('Devup UI runs on React 18', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text())
  })
  await page.goto('/')

  await expect(page.getByTestId('version')).toHaveText(/^18\./)
  await expect(page.getByTestId('refs')).toHaveText(
    'button,input,input,input,textarea,input',
  )

  await page.getByText('select', { exact: true }).click()
  await page.getByText('second', { exact: true }).click()
  await expect(page.getByTestId('selected')).toHaveText('second')

  await page.getByTestId('increase').click()
  await expect(page.getByTestId('stepper')).toHaveValue('1')

  expect(errors).toEqual([])
})
