import { invoke } from '@tauri-apps/api/core';

export interface GitStatus {
  branch: string;
  ahead: number;
  behind: number;
  changed: number;
  conflicted: string[];
}

export interface GitError {
  code: 'not_a_repository' | 'git_not_found' | 'conflict' | 'auth' | 'failed' | 'task';
  details: { message: string; files?: string[] };
}

export function gitStatus(workspacePath: string) {
  return invoke<GitStatus>('git_status', { workspacePath });
}

export function gitPull(workspacePath: string) {
  return invoke<GitStatus>('git_pull', { workspacePath });
}

export function gitSync(workspacePath: string) {
  const date = new Date();
  const now = { year: date.getFullYear(), month: date.getMonth() + 1, day: date.getDate(),
    hour: date.getHours(), minute: date.getMinutes(), second: date.getSeconds() };
  return invoke<GitStatus>('git_sync', { workspacePath, now });
}
