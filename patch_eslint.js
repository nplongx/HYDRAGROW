const fs = require('fs');

let eslintConfig = fs.readFileSync('hydragrow-frontend/eslint.config.js', 'utf8');
eslintConfig = eslintConfig.replace('"@typescript-eslint/no-explicit-any": "error",\n      "@typescript-eslint/no-explicit-any": "error",', '"@typescript-eslint/no-explicit-any": "error",');
if (!eslintConfig.includes('ignores:')) {
  eslintConfig = eslintConfig.replace('rules: {', 'ignores: ["gleam_core/build/**", "public/**", "dist/**"],\n    rules: {');
}
fs.writeFileSync('hydragrow-frontend/eslint.config.js', eslintConfig);

function replaceInFile(file, search, replace) {
  if (!fs.existsSync(file)) return;
  let content = fs.readFileSync(file, 'utf8');
  content = content.split(search).join(replace);
  fs.writeFileSync(file, content);
}

// Replace unused vars across multiple files
replaceInFile('hydragrow-frontend/src/pages/DosingHistory.tsx', 'catch (err)', 'catch');
replaceInFile('hydragrow-frontend/src/pages/RecipeBuilder.tsx', 'id: _id', 'id: _ignoreId');
replaceInFile('hydragrow-frontend/src/pages/Settings.tsx', "const [_stabilityStatus, setStabilityStatus] = useState<'idle' | 'waiting' | 'stable'>('idle');", "const [, setStabilityStatus] = useState<'idle' | 'waiting' | 'stable'>('idle');");
replaceInFile('hydragrow-frontend/src/pages/Settings.tsx', 'catch (_) {', 'catch {');
replaceInFile('hydragrow-frontend/src/pages/Settings.tsx', 'catch (error) { toast.error', 'catch { toast.error');
replaceInFile('hydragrow-frontend/src/pages/Settings.tsx', 'catch (error) { }', 'catch { }');
replaceInFile('hydragrow-frontend/src/pages/SystemLog.tsx', 'catch (err: any)', 'catch');
replaceInFile('hydragrow-frontend/src/platform/http.ts', 'timeout: _timeout, connectTimeout: _connectTimeout, ', '');
replaceInFile('hydragrow-frontend/src/platform/settings.ts', 'const { api_key: _, ...safeSettings } = parsed;', 'const { api_key, ...safeSettings } = parsed;\n      if (api_key) {}');
replaceInFile('hydragrow-frontend/test/firebase.test.ts', 'const { app } = await import', 'await import');
replaceInFile('hydragrow-frontend/vite.config.ts', "import { VitePWA } from 'vite-plugin-pwa';", "");
