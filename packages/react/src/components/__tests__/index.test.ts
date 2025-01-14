import { Box } from '../Box'
import { Button } from '../Button'
import { Center } from '../Center'
import { Flex } from '../Flex'
import { Input } from '../Input'
import { Text } from '../Text'
import { VStack } from '../VStack'

describe('Component', () => {
  it('should raise error', async () => {
    expect(() => Box({})).toThrowError('Cannot run on the runtime')
    expect(() => Button({})).toThrowError('Cannot run on the runtime')
    expect(() => Center({})).toThrowError('Cannot run on the runtime')
    expect(() => Flex({})).toThrowError('Cannot run on the runtime')
    expect(() => Input({})).toThrowError('Cannot run on the runtime')
    expect(() => Text({})).toThrowError('Cannot run on the runtime')
    expect(() => VStack({})).toThrowError('Cannot run on the runtime')
  })
})
