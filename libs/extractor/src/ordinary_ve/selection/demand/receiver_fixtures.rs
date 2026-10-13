pub(crate) const P1: &str = "export const tokens={space:'8px',read(){return ((this as typeof tokens)!).space},unused(){return this.browser},browser:window.document};";
pub(crate) const P2: &str =
    "export const tokens={space:'8px',read(){return this},browser:window.document};";
pub(crate) const P3: &str = "import {palette} from './base';export const tokens={spacing:((palette['sizes'] as {small:string;browser:unknown})!),browser:window.document};";
pub(crate) const BASE: &str =
    "export const palette={sizes:{small:'8px',browser:window.document},browser:window.navigator};";
pub(crate) const D4_SOURCE: &str =
    "const base={space:'8px'};export const tokens={...base,browser:window.document};";
pub(crate) const E1_SOURCE: &str = "import {style} from '@vanilla-extract/css';import {tokens} from './receiver';export const box=style({padding:tokens.read()});";
pub(crate) const E2_SOURCE: &str = "import {style} from '@vanilla-extract/css';import {tokens} from './receiver';export const box=style({padding:tokens.read().space});";
pub(crate) const E3_SOURCE: &str = "import {style} from '@vanilla-extract/css';import {tokens} from './forward';export const box=style({padding:tokens.spacing.small});";
