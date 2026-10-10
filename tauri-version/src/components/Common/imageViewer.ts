import './imageViewer.css';

export function openImageViewer(url: string): void {
  if (!url) return;
  document.querySelector('.q-image-viewer')?.remove();

  const root = document.createElement('div');
  root.className = 'q-image-viewer';

  const image = document.createElement('img');
  image.className = 'q-image-viewer__image';
  image.src = url;
  image.alt = '';
  image.draggable = false;
  root.append(image);

  const close = () => {
    document.removeEventListener('keydown', onKeyDown, true);
    root.remove();
  };

  function onKeyDown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    event.stopPropagation();
    close();
  }

  root.addEventListener('pointerdown', close);
  document.addEventListener('keydown', onKeyDown, true);
  document.body.append(root);
}
