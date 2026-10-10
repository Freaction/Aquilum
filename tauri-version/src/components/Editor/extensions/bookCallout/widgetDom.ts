import type { Pages } from '../../../../modules/docs/bookProgress';
import type { BookCalloutModel } from './model';
import defaultBookCover from '../../../../assets/covers/default-book-cover.webp';

export type BookCalloutViewData = BookCalloutModel & {
  pages: Pages | null;
  bookPagePath?: string;
  coverUrl: string;
  linked: boolean;
  canRead: boolean;
};

const viewDataByRoot = new WeakMap<HTMLElement, BookCalloutViewData>();

export function getBookCalloutViewData(root: HTMLElement): BookCalloutViewData | undefined {
  return viewDataByRoot.get(root);
}

function storeBookCalloutViewData(root: HTMLElement, data: BookCalloutViewData): void {
  viewDataByRoot.set(root, data);
}

function applyCoverImg(img: HTMLImageElement, coverUrl: string): void {
  const next = coverUrl.trim();
  const pending = img.dataset.coverSrc ?? null;
  if (pending === next) return;
  img.dataset.coverSrc = next;
  if (!next) {
    img.removeAttribute('src');
    return;
  }
  img.src = next;
}

function setClippedLabel(el: HTMLElement, text: string): void {
  el.textContent = text;
  el.title = text;
}

function syncProgressElement(progress: Element, pages: Pages | null): void {
  if (pages) {
    const done = pages.current >= pages.total && pages.total > 0;
    progress.classList.toggle('q-md-book-callout-progress--done', done);
    progress.textContent = `Прогресс: ${pages.current}/${pages.total}`;
  } else {
    progress.classList.remove('q-md-book-callout-progress--done');
    progress.textContent = 'Прогресс: —';
  }
}

function syncReadButton(root: HTMLElement, linked: boolean, canRead: boolean): void {
  const right = root.querySelector('.q-md-book-callout-right');
  if (!right) return;
  const read = right.querySelector('.q-md-book-callout-read') as HTMLButtonElement | null;
  if (!linked) {
    read?.remove();
    return;
  }
  const btn = read ?? (() => {
    const el = document.createElement('button');
    el.type = 'button';
    el.className = 'q-md-book-callout-read q-button q-button--xs q-button--primary';
    el.textContent = 'Читать';
    const progress = right.querySelector('.q-md-book-callout-progress');
    if (progress) right.insertBefore(el, progress);
    else right.prepend(el);
    return el;
  })();
  btn.disabled = !canRead;
  btn.title = canRead ? 'Читать' : 'Нет файла книги';
}

export function renderBookCalloutDom(data: BookCalloutViewData): HTMLElement {
  const root = document.createElement('div');
  root.className = 'q-md-book-callout';
  root.contentEditable = 'false';

  const cover = document.createElement('div');
  cover.className = 'q-md-book-callout-cover';
  const img = document.createElement('img');
  img.alt = '';
  img.draggable = false;
  applyCoverImg(img, data.coverUrl || (data.linked ? '' : defaultBookCover));
  cover.append(img);

  const text = document.createElement('div');
  text.className = 'q-md-book-callout-text';

  const title = document.createElement('div');
  title.className = data.linked
    ? 'q-md-book-callout-title q-md-book-callout-title--link'
    : 'q-md-book-callout-title';
  setClippedLabel(title, data.title);

  const author = document.createElement('div');
  author.className = 'q-md-book-callout-author';
  setClippedLabel(author, data.author || 'Автор');

  text.append(title, author);

  const right = document.createElement('div');
  right.className = 'q-md-book-callout-right';

  const progress = document.createElement('div');
  progress.className = 'q-md-book-callout-progress';
  syncProgressElement(progress, data.pages);

  right.append(progress);
  root.append(cover, text, right);
  if (data.linked) syncReadButton(root, true, data.canRead);
  storeBookCalloutViewData(root, data);
  return root;
}

export function renderBookCalloutPendingDom(): HTMLElement {
  const root = document.createElement('div');
  root.className = 'q-md-book-callout q-md-book-callout--pending';
  root.contentEditable = 'false';
  return root;
}

export function renderBookCalloutGroupDom(): HTMLElement {
  const root = document.createElement('div');
  root.className = 'q-md-book-callout-group';
  root.contentEditable = 'false';
  return root;
}

export function patchBookCalloutDom(root: HTMLElement, data: BookCalloutViewData): void {
  const img = root.querySelector('.q-md-book-callout-cover img') as HTMLImageElement | null;
  if (img) applyCoverImg(img, data.coverUrl || (data.linked ? '' : defaultBookCover));

  const title = root.querySelector('.q-md-book-callout-title') as HTMLElement | null;
  if (title) {
    setClippedLabel(title, data.title);
    title.classList.toggle('q-md-book-callout-title--link', data.linked);
  }

  const author = root.querySelector('.q-md-book-callout-author') as HTMLElement | null;
  if (author) setClippedLabel(author, data.author || 'Автор');

  const progress = root.querySelector('.q-md-book-callout-progress');
  if (progress) syncProgressElement(progress, data.pages);

  syncReadButton(root, data.linked, data.canRead);
  storeBookCalloutViewData(root, data);
}
