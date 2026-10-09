export const shorthands = {
  mixedScale: ['padding', 'opacity', 'animation-duration'],
}
const absent = [
  'null',
  'undefined',
  'false',
  'true',
  "''",
  'NaN',
  'Infinity',
  '-Infinity',
]
export const additionalFixtures = [
  ...[
    ['proven-number', '0.5', ':number'],
    ['numeric-capable-string', "'0.5'", ':string'],
    ['unknown', '0.5', ''],
  ].map(([name, value, annotation]) => ({
    name: `mixedScale-${name}`,
    dynamic: '<Box mixedScale={n}/>',
    literal: '<Box p={0.5} opacity={0.5} animationDuration={0.5}/>',
    value,
    annotation,
    props: ['paddingTop', 'opacity', 'animationDuration'],
    expected: ['2px', '0.5', '0.0005s'],
  })),
]
export const isolatedSuites = []
for (const [prop, computed, anchor, expected] of [
  ['p', 'paddingTop', 'anchor', '7px'],
  ['opacity', 'opacity', 'opacityAnchor', '0.37'],
]) {
  const isolated = []
  for (const value of absent) {
    for (const annotation of ['', ':number']) {
      const fixture = {
        name: `absent-plain-${prop}-${annotation ? 'number' : 'unknown'}-${value}`,
        dynamic: `<Box ${prop}={n}/>`,
        literal: '<Box/>',
        value,
        annotation,
        props: [computed],
        expected: [prop === 'p' ? '0px' : '1'],
      }
      additionalFixtures.push(fixture)
      isolated.push({ ...fixture, name: `${fixture.name}-noOtherRule` })
      if (prop === 'opacity')
        additionalFixtures.push({
          ...fixture,
          name: `absent-prior-opacity-${annotation ? 'number' : 'unknown'}-${value}`,
          dynamic: `<Box className="${anchor}" opacity={n}/>`,
          literal: `<Box className="${anchor}"/>`,
          expected: [expected],
        })
    }
  }
  isolatedSuites.push({
    name: `noOtherRule-${prop}`,
    fixtures: isolated,
    source: `import {Box} from '@devup-ui/react';${isolated
      .map(
        (fixture, index) =>
          `function dynamic${index}(n${fixture.annotation}){return ${fixture.dynamic}}globalThis.pairs.push([dynamic${index}(${fixture.value}),${fixture.literal}]);`,
      )
      .join('\n')}`,
    importedSource: "import {Box} from '@devup-ui/react';",
    baselineCss: '',
  })
}
