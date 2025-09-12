import {
    Box, 
    Button as DevupButton,
    Text as DevupText, 
    Center,
    css, 
    type DevupThemeTypography
} from '@devup-ui/react'
import { clsx } from 'clsx'

type LabelProps = React.LabelHTMLAttributes<HTMLLabelElement> & {
    // our custom properties. 
}

const lableTextEllipsis = css({
    // TODO 
})

export function Label({
    // TODO 
}: LabelProps): React.ReactElement {
    return (
        <DevupText>
            TODO 
        </DevupText>
    )
}