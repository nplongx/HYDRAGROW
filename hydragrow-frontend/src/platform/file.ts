import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';

const isTauriEnvironment = () => {
  if (typeof window === 'undefined') return false;
  return '__TAURI_INTERNALS__' in window;
};

export const saveTextFile = async (filename: string, content: string): Promise<void> => {
  if (isTauriEnvironment()) {
    const filePath = await save({ defaultPath: filename });

    if (!filePath) {
      return;
    }

    await writeTextFile(filePath, content);
    return;
import { isTauriRuntime } from './settings';

export const saveTextFile = async (defaultPath: string, content: string): Promise<boolean> => {
  if (isTauriRuntime()) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const { writeTextFile } = await import('@tauri-apps/plugin-fs');
    const filePath = await save({ defaultPath });
    if (!filePath) return false;
    await writeTextFile(filePath, content);
    return true;
  }

  const blob = new Blob([content], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.setAttribute('download', defaultPath);
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
  return true;
};
