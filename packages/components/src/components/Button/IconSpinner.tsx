import { Box, keyframes } from '@devup-ui/react'
import { SVGProps } from 'react'

const spin = keyframes({
  '0%': {
    transform: 'rotateZ(0deg)',
  },
  '100%': {
    transform: 'rotateZ(360deg)',
  },
})

interface IconSpinnerProps extends SVGProps<SVGSVGElement> {
  type?: 'whole' | 'partial'
}

export function IconSpinner({ type = 'whole', ...props }: IconSpinnerProps) {
  if (type === 'partial') {
    return (
      <Box
        animationDuration="1s"
        animationIterationCount="infinite"
        animationName={spin}
        animationTimingFunction="linear"
        aria-label="Partial loading spinner"
        as="svg"
        props={{
          fill: 'none',
          height: '20',
          viewBox: '0 0 20 20',
          width: '20',
          xmlns: 'http://www.w3.org/2000/svg',
          ...props,
        }}
      >
        <path
          d="M17 10C17 11.291 16.643 12.5568 15.9685 13.6575C15.294 14.7582 14.3282 15.651 13.1779 16.237C12.0277 16.8231 10.7378 17.0797 9.45078 16.9784C8.1638 16.8771 6.9299 16.4219 5.8855 15.6631"
          stroke="light-dark(var(--primary, #272727), var(--primary, #F6F6F6))"
          strokeLinecap="round"
          strokeWidth="3"
        />
      </Box>
    )
  }
  return (
    <Box
      animationDuration="1s"
      animationIterationCount="infinite"
      animationName={spin}
      animationTimingFunction="linear"
      aria-label="Whole loading spinner"
      as="svg"
      props={{
        fill: 'none',
        height: '20',
        viewBox: '0 0 20 20',
        width: '20',
        xmlns: 'http://www.w3.org/2000/svg',
        ...props,
      }}
    >
      <g clipPath="url(#paint0_angular_1842_200_clip_path)">
        <g transform="matrix(0 0.007 -0.007 0 10 10)">
          <foreignObject
            height="2857.14"
            width="2857.14"
            x="-1428.57"
            y="-1428.57"
          >
            <div
              style={{
                background:
                  'conic-gradient(from 90deg, light-dark(var(--primary, #272727), var(--primary, #F6F6F6)) 0deg,rgba(0,0,0,0) 360deg)',
                height: '100%',
                width: '100%',
                opacity: 1,
              }}
            ></div>
          </foreignObject>
        </g>
      </g>
      <path d="M17 10H15.5C15.5 13.0376 13.0376 15.5 10 15.5V17V18.5C14.6944 18.5 18.5 14.6944 18.5 10H17ZM10 17V15.5C6.96243 15.5 4.5 13.0376 4.5 10H3H1.5C1.5 14.6944 5.30558 18.5 10 18.5V17ZM3 10H4.5C4.5 6.96243 6.96243 4.5 10 4.5V3V1.5C5.30558 1.5 1.5 5.30558 1.5 10H3ZM10 3V4.5C13.0376 4.5 15.5 6.96243 15.5 10H17H18.5C18.5 5.30558 14.6944 1.5 10 1.5V3Z" />
      <defs>
        <clipPath id="paint0_angular_1842_200_clip_path">
          <path d="M17 10H15.5C15.5 13.0376 13.0376 15.5 10 15.5V17V18.5C14.6944 18.5 18.5 14.6944 18.5 10H17ZM10 17V15.5C6.96243 15.5 4.5 13.0376 4.5 10H3H1.5C1.5 14.6944 5.30558 18.5 10 18.5V17ZM3 10H4.5C4.5 6.96243 6.96243 4.5 10 4.5V3V1.5C5.30558 1.5 1.5 5.30558 1.5 10H3ZM10 3V4.5C13.0376 4.5 15.5 6.96243 15.5 10H17H18.5C18.5 5.30558 14.6944 1.5 10 1.5V3Z" />
        </clipPath>
      </defs>
    </Box>
  )
}
