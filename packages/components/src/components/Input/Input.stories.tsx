import { Meta, StoryObj } from '@storybook/react-vite'

import { Input } from './index'

type Story = StoryObj<typeof meta>

// More on how to set up stories at: https://storybook.js.org/docs/writing-stories#default-export
const meta: Meta<typeof Input> = {
  title: 'Devfive/Input',
  component: Input,
  decorators: [
    (Story) => (
      <div style={{ padding: '10px' }}>
        <Story />
      </div>
    ),
  ],
}

export const Default: Story = {
  args: {
    placeholder: 'Input text',
  },
}

export const Error: Story = {
  args: {
    placeholder: 'Input text',
    error: true,
    errorMessage: 'Error message',
  },
}

export const Disabled: Story = {
  args: {
    placeholder: 'Input text',
    disabled: true,
  },
}

export const WithIcon: Story = {
  args: {
    placeholder: 'Input text',
    icon: (
      <svg
        fill="none"
        height="24"
        viewBox="0 0 24 24"
        width="24"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          clipRule="evenodd"
          d="M14.5006 15.9949C13.445 16.6754 12.1959 17.069 10.8571 17.069C7.07005 17.069 4 13.9195 4 10.0345C4 6.14945 7.07005 3 10.8571 3C14.6442 3 17.7143 6.14945 17.7143 10.0345C17.7143 11.7044 17.1471 13.2384 16.1995 14.4448C16.2121 14.4567 16.2245 14.4688 16.2367 14.4813L19.6653 17.9986C20.1116 18.4564 20.1116 19.1988 19.6653 19.6566C19.2189 20.1145 18.4953 20.1145 18.049 19.6566L14.6204 16.1394C14.5761 16.0938 14.5361 16.0455 14.5006 15.9949ZM16.2143 10.0345C16.2143 13.1274 13.7799 15.569 10.8571 15.569C7.93435 15.569 5.5 13.1274 5.5 10.0345C5.5 6.94154 7.93435 4.5 10.8571 4.5C13.7799 4.5 16.2143 6.94154 16.2143 10.0345Z"
          fill="#8D8C9A"
          fillRule="evenodd"
        />
      </svg>
    ),
  },
}

// export const WithForm: Story = {
//   args: {
//     children: 'Input text',
//     type: 'submit',
//   },
//   decorators: [
//     (Story, { args }: { args: Story['args'] }) => {
//       const [submitted, setSubmitted] = useState<{ text?: string }>({})
//       const [value, setValue] = useState('')
//       const [error, setError] = useState('')

//       return (
//         <>
//           <div>{submitted.text}</div>
//           <form
//             onSubmit={(e) => {
//               e.preventDefault()
//               const formData = new FormData(e.target as HTMLFormElement)
//               const data = Object.fromEntries(formData)

//               setSubmitted({
//                 text: data.text as string,
//               })
//             }}
//           >
//             <input
//               className={css({
//                 display: 'block',
//                 mb: '10px',
//               })}
//               minLength={3}
//               name="text"
//               onChange={(e) => {
//                 setValue(e.target.value)
//                 setError(
//                   !/[0-9]/.test(e.target.value) && e.target.value.length >= 3
//                     ? 'Include one or more numbers.'
//                     : '',
//                 )
//               }}
//               placeholder="Include one or more numbers."
//               required
//               type="text"
//             />
//             <Story
//               args={{
//                 ...args,
//                 disabled: value.length < 3,
//                 danger: !!error,
//               }}
//             />
//           </form>
//         </>
//       )
//     },
//   ],
// }

export default meta
