import { style } from '@vanilla-extract/css'

export const container = style({
  padding: '2rem',
  backgroundColor: '#f0f0f0',
  borderRadius: '8px',
  marginBottom: '1rem',
})

export const title = style({
  fontSize: '2rem',
  fontWeight: 'bold',
  color: '#333',
  marginBottom: '1rem',
})

export const card = style({
  backgroundColor: 'white',
  padding: '1.5rem',
  borderRadius: '4px',
  boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
  marginBottom: '1rem',
  transition: 'transform 0.2s ease',
  ':hover': {
    transform: 'translateY(-2px)',
    boxShadow: '0 4px 8px rgba(0, 0, 0, 0.15)',
  },
})

export const button = style({
  backgroundColor: '#007bff',
  color: 'white',
  border: 'none',
  padding: '0.75rem 1.5rem',
  borderRadius: '4px',
  fontSize: '1rem',
  cursor: 'pointer',
  transition: 'background-color 0.2s ease',
  ':hover': {
    backgroundColor: '#0056b3',
  },
  ':active': {
    backgroundColor: '#004494',
  },
})

export const text = style({
  fontSize: '1rem',
  lineHeight: 1.6,
  color: '#666',
})
