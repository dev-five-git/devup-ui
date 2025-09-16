import { SVGProps } from 'react'

type CheckIconProps = SVGProps<SVGSVGElement> & {
  color: string
}

export function CheckIcon({ color, ...props }: CheckIconProps) {
  return (
    <svg
      height="10"
      viewBox="0 0 12 10"
      width="12"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        clipRule="evenodd"
        d="M11.6474 0.807113C12.0992 1.23373 12.1195 1.94575 11.6929 2.39745L5.31789 9.14745C5.10214 9.37589 4.80069 9.50369 4.48649 9.49992C4.1723 9.49615 3.874 9.36114 3.6638 9.12759L0.288803 5.37759C-0.126837 4.91576 -0.0893993 4.20444 0.372424 3.7888C0.834247 3.37315 1.54557 3.41059 1.96121 3.87242L4.51994 6.71544L10.0571 0.852551C10.4837 0.400843 11.1957 0.3805 11.6474 0.807113Z"
        fill={color}
        fillRule="evenodd"
      />
    </svg>
  )
}
