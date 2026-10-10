import { fileName } from '../paths';
import { createTracer } from './trace';

type OpenStage =
  | 'request'
  | 'doc'
  | 'mount'
  | 'scroll'
  | 'links'
  | 'images'
  | 'callouts'
  | 'dataview';

const tracer = createTracer<OpenStage>({
  name: 'open',
  origin: 'now',
  order: ['request', 'doc', 'mount', 'scroll', 'links', 'images', 'callouts', 'dataview'],
  labels: {
    request: 'интерфейс запросил документ у ядра',
    doc: 'ядро открыло документ и сверило его с файлом',
    mount: 'редактор смонтирован',
    scroll: 'прокрутка восстановлена',
    links: 'вики-ссылки разрешены',
    images: 'вложения разрешены',
    callouts: 'книжные строки',
    dataview: 'запросы dataview',
  },
});

export function beginOpenTrace(path: string): void {
  tracer.begin(fileName(path));
}

export function markOpenStage(stage: OpenStage): void {
  tracer.mark(stage);
}
