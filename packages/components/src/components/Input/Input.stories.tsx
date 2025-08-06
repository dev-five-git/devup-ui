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

// export const WithIcon: Story = {
//   args: {
//     disabled: true,
//     icon: (
//       <svg
//         className={css({ color: '$text' })}
//         fill="none"
//         height="24"
//         viewBox="0 0 24 24"
//         width="24"
//         xmlns="http://www.w3.org/2000/svg"
//       >
//         <path
//           clipRule="evenodd"
//           d="M16.2635 4.3205C15.8763 3.90288 15.2518 3.89202 14.8523 4.29596L6.92714 12.3101C6.77288 12.4661 6.66766 12.6701 6.6262 12.8938L6.19139 15.2388C6.04942 16.0044 6.67528 16.6795 7.38514 16.5264L9.56701 16.0557C9.74988 16.0163 9.91913 15.9232 10.0562 15.7868L16.6085 9.26287C16.6164 9.25496 16.6242 9.24687 16.6319 9.23862L18.0101 7.75198C18.4063 7.32464 18.4063 6.63179 18.0101 6.20445L16.2635 4.3205ZM15.1465 6.39842L15.5325 6.00805L16.4319 6.97821L16.058 7.38159L15.1465 6.39842ZM13.9617 7.59651L14.8868 8.59436L9.08091 14.3751L7.96212 14.6164L8.17961 13.4435L13.9617 7.59651ZM5.91304 18.0303C5.40878 18.0303 5 18.4712 5 19.0152C5 19.5591 5.40878 20 5.91304 20H18.087C18.5912 20 19 19.5591 19 19.0152C19 18.4712 18.5912 18.0303 18.087 18.0303H5.91304Z"
//           fill="currentColor"
//           fillRule="evenodd"
//         />
//       </svg>
//     ),
//   },
// }

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
