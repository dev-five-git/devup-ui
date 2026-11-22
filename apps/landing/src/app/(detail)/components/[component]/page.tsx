import { COMPONENT_GROUPS } from '@/constants'

import {
  ComponentDoc,
  getComponentMetadata,
} from '../ComponentDoc'

export const generateMetadata = async ({
  params,
}: {
  params: Promise<{ component: string }>
}) => getComponentMetadata((await params).component)

export const generateStaticParams = async () => {
  return Object.values(COMPONENT_GROUPS)
    .flat()
    .map((component) => ({
      component,
    }))
}

export default async function Page({
  params,
}: {
  params: Promise<{ component: string }>
}) {
  const { component } = await params
  const { default: Index } = await import(`./${component}/index.mdx`)
  const { default: Api } = await import(`./${component}/Api.mdx`)

  return <ComponentDoc Api={Api} component={component} Index={Index} />
}
