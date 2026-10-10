import { EditorView } from '@codemirror/view';

export const imageEmbedTheme = EditorView.baseTheme({
  '.q-md-image': {
    display: 'block',
    boxSizing: 'border-box',
    width: '0',
    minWidth: '100%',
    padding: 'var(--q-space-8) var(--q-space-4)',
    lineHeight: '0',
    userSelect: 'none',
  },
  '.q-md-image-wrap': {
    position: 'relative',
    width: '100%',
    maxWidth: '100%',
  },
  '.q-md-image[data-align="center"] .q-md-image-wrap': {
    marginInline: 'auto',
  },
  '.q-md-image[data-align="left"] .q-md-image-wrap': {
    marginRight: 'auto',
  },
  '.q-md-image[data-align="right"] .q-md-image-wrap': {
    marginLeft: 'auto',
  },
  '.q-md-image-clip': {
    position: 'relative',
    overflow: 'hidden',
    borderRadius: 'var(--q-radius-xl)',
  },
  '.q-md-image[data-selected="true"] .q-md-image-clip': {
    boxShadow: 'var(--q-shadow-focus-ring)',
    outline: 'var(--q-border-1) solid var(--q-border-focus)',
    outlineOffset: 'calc(-1 * var(--q-border-1))',
  },
  '.q-md-image-img': {
    display: 'block',
    width: '100%',
    height: 'auto',
  },
  '.q-md-image-toolbar': {
    position: 'absolute',
    top: 'var(--q-space-8)',
    right: 'var(--q-space-8)',
  },
  '.q-md-image-crop': {
    position: 'absolute',
    inset: '0',
  },
  '.q-md-image-crop__shade': {
    position: 'absolute',
    inset: '0',
    overflow: 'hidden',
    borderRadius: 'var(--q-radius-xl)',
    pointerEvents: 'none',
  },
  '.q-md-image-crop__dim': {
    position: 'absolute',
    boxShadow: '0 0 0 100vmax var(--q-bg-overlay)',
  },
  '.q-md-image-crop__box': {
    position: 'absolute',
    outline: 'var(--q-border-1) solid var(--q-border-contrast-inverse)',
    cursor: 'move',
  },
  '.q-md-image-crop__handle': {
    position: 'absolute',
    width: 'var(--q-size-10)',
    height: 'var(--q-size-10)',
    borderRadius: 'var(--q-radius-full)',
    background: 'var(--q-icon-accent)',
  },
  '.q-md-image-crop__handle[data-edge="nw"]': {
    left: '0',
    top: '0',
    transform: 'translate(-50%, -50%)',
    cursor: 'nwse-resize',
  },
  '.q-md-image-crop__handle[data-edge="n"]': {
    left: '50%',
    top: '0',
    transform: 'translate(-50%, -50%)',
    cursor: 'ns-resize',
  },
  '.q-md-image-crop__handle[data-edge="ne"]': {
    right: '0',
    top: '0',
    transform: 'translate(50%, -50%)',
    cursor: 'nesw-resize',
  },
  '.q-md-image-crop__handle[data-edge="e"]': {
    right: '0',
    top: '50%',
    transform: 'translate(50%, -50%)',
    cursor: 'ew-resize',
  },
  '.q-md-image-crop__handle[data-edge="se"]': {
    right: '0',
    bottom: '0',
    transform: 'translate(50%, 50%)',
    cursor: 'nwse-resize',
  },
  '.q-md-image-crop__handle[data-edge="s"]': {
    left: '50%',
    bottom: '0',
    transform: 'translate(-50%, 50%)',
    cursor: 'ns-resize',
  },
  '.q-md-image-crop__handle[data-edge="sw"]': {
    left: '0',
    bottom: '0',
    transform: 'translate(-50%, 50%)',
    cursor: 'nesw-resize',
  },
  '.q-md-image-crop__handle[data-edge="w"]': {
    left: '0',
    top: '50%',
    transform: 'translate(-50%, -50%)',
    cursor: 'ew-resize',
  },
  '.q-md-image-toolbar .q-floating-actions__divider': {
    height: 'var(--q-size-16)',
  },
  '.q-md-image .q-md-image-grip': {
    position: 'absolute',
    right: 'var(--q-space-8)',
    bottom: 'var(--q-space-8)',
    cursor: 'nwse-resize',
    animation: 'q-floating-actions-in var(--q-duration-fast) var(--q-ease-out)',
  },
});
