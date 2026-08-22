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
