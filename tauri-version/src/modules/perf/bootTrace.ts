import { createTracer } from './trace';

type BootStage =
  | 'scripts'
  | 'render'
  | 'shell'
  | 'workspace'
  | 'session'
  | 'config';

const tracer = createTracer<BootStage>({
  name: 'boot',
  origin: 'load',
  order: ['scripts', 'render', 'shell', 'workspace', 'session', 'config'],
  labels: {
    scripts: 'модули загружены и выполнены',
    render: 'React получил дерево',
    shell: 'первый кадр приложения',
    workspace: 'база знаний известна',
    session: 'сессия вкладок загружена',
    config: 'настройки загружены, окно показано',
  },
});

export function beginBootTrace(): void {
  tracer.begin('запуск');
}

export function markBootStage(stage: BootStage): void {
  tracer.mark(stage);
}
