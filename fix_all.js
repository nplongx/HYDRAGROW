const fs = require('fs');

function replaceInFile(file, search, replace) {
  if (!fs.existsSync(file)) return;
  let content = fs.readFileSync(file, 'utf8');
  content = content.split(search).join(replace);
  fs.writeFileSync(file, content);
}

replaceInFile('hydragrow-frontend/src/pages/DevicePairing.tsx', ': any', ': unknown');
replaceInFile('hydragrow-frontend/src/pages/SystemLog.tsx', ': any', ': unknown');
replaceInFile('hydragrow-frontend/src/pages/ConfigBackup.tsx', ': any', ': unknown');
replaceInFile('hydragrow-frontend/src/pages/UserManagement.tsx', ': any', ': unknown');
replaceInFile('hydragrow-frontend/src/pages/ControlPanel.tsx', ': any', ': unknown');
replaceInFile('hydragrow-frontend/src/pages/Dashboard.tsx', ': any', ': unknown');
replaceInFile('hydragrow-frontend/src/pages/Settings.tsx', ': any', ': unknown');
replaceInFile('hydragrow-frontend/src/pages/DosingHistory.tsx', ': any', ': unknown');

// http.ts
replaceInFile('hydragrow-frontend/src/platform/http.ts', ': any', ': unknown');

// store/useDeviceStore.test.ts
replaceInFile('hydragrow-frontend/src/store/useDeviceStore.test.ts', ': any', ': unknown');

// components
const glob = require('glob');
try {
  const components = glob.sync('hydragrow-frontend/src/components/**/*.tsx');
  components.forEach(c => replaceInFile(c, ': any', ': unknown'));
} catch (e) {
  // Ignore glob error if not installed, will fallback to explicit
  const compFiles = [
    'hydragrow-frontend/src/components/seasons/CreateSeasonForm.tsx',
    'hydragrow-frontend/src/components/seasons/ActiveSeasonCard.tsx',
    'hydragrow-frontend/src/components/logs/MetadataRenderers.tsx',
    'hydragrow-frontend/src/components/logs/EventLogCard.tsx',
    'hydragrow-frontend/src/components/layout/MainLayout.tsx',
    'hydragrow-frontend/src/components/dosing/DosingReportCard.tsx',
    'hydragrow-frontend/src/hooks/useFCM.ts',
    'hydragrow-frontend/src/hooks/useFleetStatus.ts',
    'hydragrow-frontend/src/hooks/useDeviceSync.ts',
    'hydragrow-frontend/src/hooks/useDeviceControl.ts',
    'hydragrow-frontend/src/hooks/useCropSeason.ts',
    'hydragrow-frontend/src/lib/apiClient.ts',
    'hydragrow-frontend/src/contexts/AuthContext.tsx'
  ];
  compFiles.forEach(c => replaceInFile(c, ': any', ': unknown'));
}

// Ensure gleam build dir is fully ignored
let eslintConfig = fs.readFileSync('hydragrow-frontend/eslint.config.js', 'utf8');
if (!eslintConfig.includes('ignores:')) {
  eslintConfig = eslintConfig.replace('rules: {', 'ignores: ["gleam_core/build/**", "dist/**"],\n    rules: {');
} else {
  // If it does include it, make sure it has gleam_core/build/**
  if (!eslintConfig.includes('gleam_core/build/**')) {
      eslintConfig = eslintConfig.replace('ignores: [', 'ignores: ["gleam_core/build/**", ');
  }
}
fs.writeFileSync('hydragrow-frontend/eslint.config.js', eslintConfig);
