import { globalCss } from '@devup-ui/react'

globalCss({
  '*,:after,:before': {
    boxSizing: 'border-box',
    backgroundRepeat: 'no-repeat',
  },
  ':after,:before': {
    textDecoration: 'inherit',
    verticalAlign: 'inherit',
  },
  ':where(:root)': {
    cursor: 'default',
    lineHeight: 1.5,
    overflowWrap: 'break-word',
    MozTabSize: 4,
    tabSize: 4,
    WebkitTapHighlightColor: 'transparent',
    WebkitTextSizeAdjust: '100%',
  },
  ':where(body)': {
    m: 0,
  },
  ':where(h1)': {
    fontSize: '2em',
    m: '.67em 0',
  },
  ':where(dl,ol,ul) :where(dl,ol,ul)': {
    m: 0,
  },
  ':where(hr)': {
    color: 'inherit',
    h: 0,
  },
  ':where(nav) :where(ol,ul)': {
    listStyleType: 'none',
    p: 0,
  },
  ':where(nav li):before': {
    content: '"\\200B"',
    float: 'left',
  },
  ':where(pre)': {
    fontFamily: 'monospace,monospace',
    fontSize: '1em',
    overflow: 'auto',
  },
  ':where(abbr[title])': {
    textDecoration: 'underline dotted',
    WebkitTextDecorationStyle: 'dotted',
    WebkitTextDecorationLine: 'underline',
  },
  ':where(b,strong)': {
    fontWeight: 'bolder',
  },
  ':where(code,kbd,samp)': {
    fontFamily: 'monospace,monospace',
    fontSize: '1em',
  },
  ':where(small)': {
    fontSize: '80%',
  },
  ':where(audio,canvas,iframe,img,svg,video)': {
    verticalAlign: 'middle',
  },
  ':where(iframe)': {
    borderStyle: 'none',
  },
  ':where(svg:not([fill]))': {
    fill: 'currentColor',
  },
  ':where(table)': {
    borderCollapse: 'collapse',
    borderColor: 'inherit',
    textIndent: 0,
  },
  ':where(button,input,select)': {
    m: 0,
  },
  ':where(button,[type=button i],[type=reset i],[type=submit i])': {
    WebkitAppearance: 'button',
  },
  ':where(fieldset)': {
    border: '1px solid #a0a0a0',
  },
  ':where(progress)': {
    verticalAlign: 'baseline',
  },
  ':where(textarea)': {
    m: 0,
    resize: 'vertical',
  },
  ':where([type=search i])': {
    WebkitAppearance: 'textfield',
    outlineOffset: '-2px',
  },
  '::-webkit-inner-spin-button,::-webkit-outer-spin-button': {
    height: 'auto',
  },
  '::-webkit-input-placeholder': {
    color: 'inherit',
    opacity: 0.54,
  },
  '::-webkit-search-decoration': {
    WebkitAppearance: 'none',
  },
  '::-webkit-file-upload-button': {
    WebkitAppearance: 'button',
    font: 'inherit',
  },
  ':where(dialog)': {
    bgColor: 'white',
    border: 'solid',
    color: 'black',
    left: 0,
    m: 'auto',
    p: '1em',
    pos: 'absolute',
    right: 0,
    w: 'fit-content',
    h: 'fit-content',
  },
  ':where(dialog:not([open]))': {
    display: 'none',
  },
  ':where(details>summary:first-of-type)': {
    display: 'list-item',
  },
  ':where([aria-busy=true i])': {
    cursor: 'progress',
  },
  ':where([aria-controls])': {
    cursor: 'pointer',
  },
  ':where([aria-disabled=true i],[disabled])': {
    cursor: 'not-allowed',
  },
  ':where([aria-hidden=false i][hidden])': {
    display: 'initial',
  },
  ':where([aria-hidden=false i][hidden]:not(:focus))': {
    pos: 'absolute',
  },
})

export function resetCss(): void {}
