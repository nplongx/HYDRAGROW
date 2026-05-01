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
  }

  const blob = new Blob([content], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');

  link.href = url;
  link.setAttribute('download', filename);
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
};
