import type { Type, TypeChecker } from 'typescript'

import { isPassThroughProp } from '../../utils/style-position'
import { domProperties } from './dom-properties'
import { keyDomainsOverlapExcept } from './key-domain'

export class StyleSchemaError extends Error {
  readonly name = 'StyleSchemaError'

  constructor(readonly filename: string) {
    super(
      `Cannot resolve Devup component prop types in ${filename}. Check @devup-ui/react and React type resolution in the lint project.`,
    )
  }
}

/** The component's public props supply the schema, including theme/custom augmentation. */
export function styleCapability(
  checker: TypeChecker,
  component: Type,
  filename: string,
) {
  const signatures = checker.getSignaturesOfType(component, 0)
  const signature = signatures[0]
  const parameter = signature?.getParameters()[0]
  const declaration = signature?.getDeclaration()
  if (!signature || !parameter || !declaration)
    throw new StyleSchemaError(filename)
  const props = checker.getTypeOfSymbolAtLocation(parameter, declaration)
  const domDeclarations = domProperties(
    checker,
    checker.getReturnTypeOfSignature(signature),
  )
  if (props.flags & checker.getAnyType().flags || !domDeclarations?.size)
    throw new StyleSchemaError(filename)
  const domOrigin = (
    property: ReturnType<TypeChecker['getPropertiesOfType']>[number],
  ) => {
    const declarations = property.getDeclarations()
    return Boolean(
      declarations?.length &&
      declarations.every((node) => domDeclarations.has(node)),
    )
  }
  const keys = new Set(
    checker
      .getPropertiesOfType(props)
      .filter(
        (property) => !isPassThroughProp(property.name) && !domOrigin(property),
      )
      .map((property) => property.name),
  )
  const indexes = checker.getIndexInfosOfType(props)
  const styleKey = (name: string) =>
    !isPassThroughProp(name) &&
    (keys.has(name) ||
      indexes.some((index) =>
        checker.isTypeAssignableTo(
          checker.getStringLiteralType(name),
          index.keyType,
        ),
      ))
  const visiting = new Set<Type>()
  const emits = (type: Type) =>
    checker.getNonNullableType(type) !== checker.getNeverType()
  const capable = (type: Type, excluded: readonly Type[] = []): boolean => {
    if (
      type.flags &
      (checker.getAnyType().flags | checker.getUnknownType().flags)
    )
      return true
    if (visiting.has(type)) return false
    visiting.add(type)
    const intersection = type.isIntersection() ? type : undefined
    let result: boolean
    if (type.isUnion()) {
      result = type.types.some((part) => capable(part, excluded))
    } else if (type.isTypeParameter()) {
      const constraint = checker.getBaseConstraintOfType(type)
      result = constraint ? capable(constraint, excluded) : true
    } else {
      const properties = checker.getPropertiesOfType(type)
      const typeIndexes = checker.getIndexInfosOfType(type)
      const blocked = [
        ...excluded,
        ...properties
          .filter(
            (property) =>
              !emits(checker.getTypeOfSymbolAtLocation(property, declaration)),
          )
          .map((property) => checker.getStringLiteralType(property.name)),
        ...typeIndexes
          .filter((index) => !emits(index.type))
          .map((index) => index.keyType),
      ]
      const available = (key: Type) =>
        !blocked.some((domain) => checker.isTypeAssignableTo(key, domain))
      result =
        (properties.length === 0 &&
          typeIndexes.length === 0 &&
          checker.isTypeAssignableTo(props, type)) ||
        properties.some(
          (property) =>
            styleKey(property.name) &&
            !domOrigin(property) &&
            available(checker.getStringLiteralType(property.name)),
        ) ||
        typeIndexes.some(
          (candidate) =>
            emits(candidate.type) &&
            ([...keys].some((key) => {
              const literal = checker.getStringLiteralType(key)
              return (
                checker.isTypeAssignableTo(literal, candidate.keyType) &&
                available(literal)
              )
            }) ||
              indexes.some((index) =>
                keyDomainsOverlapExcept(
                  checker,
                  [index.keyType, candidate.keyType],
                  blocked,
                ),
              )),
        ) ||
        (intersection?.types.some(
          (part) => part.isTypeParameter() && capable(part, blocked),
        ) ??
          false)
    }
    visiting.delete(type)
    return result
  }
  return capable
}
