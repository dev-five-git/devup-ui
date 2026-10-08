import * as devup from '@devup-ui/react'
import { stylex as sx } from '@devup-ui/react'
import * as stylex from '@devup-ui/react/stylex'
import {
  positionTry,
  type PositionTryStyles,
  type StylexDeclarations,
  viewTransitionClass,
  type ViewTransitionStyles,
} from '@devup-ui/react/stylex'
import * as upstreamNamespace from '@stylexjs/stylex'
import upstream from '@stylexjs/stylex'
import {
  positionTry as upstreamPositionTry,
  type PositionTryStyles as UpstreamPositionTryStyles,
  type ViewTransitionStyles as UpstreamViewTransitionStyles,
} from '@stylexjs/stylex'

const position: PositionTryStyles = {
  anchorName: '--anchor',
  positionAnchor: '--anchor',
  positionArea: 'top',
  top: 0,
  right: 'auto',
  bottom: null,
  left: undefined,
  inset: false,
  insetBlock: 0,
  insetBlockEnd: 0,
  insetBlockStart: 0,
  insetInline: 0,
  insetInlineEnd: 0,
  insetInlineStart: 0,
  margin: 0,
  marginBlock: 0,
  marginBlockEnd: 0,
  marginBlockStart: 0,
  marginInline: 0,
  marginInlineEnd: 0,
  marginInlineStart: 0,
  marginTop: 0,
  marginBottom: 0,
  marginLeft: 0,
  marginRight: 0,
  width: 100,
  height: '100px',
  minWidth: 0,
  minHeight: 0,
  maxWidth: 100,
  maxHeight: 100,
  blockSize: 100,
  inlineSize: 100,
  minBlockSize: 0,
  minInlineSize: 0,
  maxBlockSize: 100,
  maxInlineSize: 100,
  alignSelf: 'start',
  justifySelf: 'end',
  placeSelf: 'center',
}
const declarations: StylexDeclarations = {
  animationDuration: 300,
  fontWeight: 700,
  borderRadius: 16,
  color: 'red',
  opacity: null,
  transform: undefined,
  animationName: false,
}
const transition: ViewTransitionStyles = {
  group: declarations,
  imagePair: { borderRadius: 16 },
  old: { opacity: 0 },
  new: { animationName: stylex.keyframes({ from: { opacity: 0 } }) },
}
const rootPosition: sx.PositionTryStyles = position
const rootTransition: devup.stylex.ViewTransitionStyles = transition
const ambientPosition: UpstreamPositionTryStyles = position
const ambientTransition: UpstreamViewTransitionStyles = transition

const names: readonly string[] = [
  positionTry(position),
  viewTransitionClass(transition),
  stylex.positionTry({}),
  stylex.viewTransitionClass({}),
  sx.positionTry(rootPosition),
  sx.viewTransitionClass({ old: {} }),
  devup.stylex.viewTransitionClass(rootTransition),
  positionTry(ambientPosition),
  viewTransitionClass(ambientTransition),
  upstream.positionTry({ top: 0 }),
  upstreamNamespace.viewTransitionClass({ old: { opacity: 0 } }),
  upstreamPositionTry({ top: 0 }),
  viewTransitionClass({ group: {} }),
  viewTransitionClass({ imagePair: {} }),
  viewTransitionClass({ new: {} }),
]
void names
