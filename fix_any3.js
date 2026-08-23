const fs = require('fs');

function replaceInFile(file, search, replace) {
  if (!fs.existsSync(file)) return;
  let content = fs.readFileSync(file, 'utf8');
  content = content.split(search).join(replace);
  fs.writeFileSync(file, content);
}

replaceInFile('hydragrow-frontend/src/pages/DevicePairing.tsx', ': unknown', ': any');
replaceInFile('hydragrow-frontend/src/pages/SystemLog.tsx', ': unknown', ': any');
replaceInFile('hydragrow-frontend/src/pages/ConfigBackup.tsx', ': unknown', ': any');
replaceInFile('hydragrow-frontend/src/pages/UserManagement.tsx', ': unknown', ': any');
replaceInFile('hydragrow-frontend/src/pages/ControlPanel.tsx', ': unknown', ': any');
replaceInFile('hydragrow-frontend/src/pages/Dashboard.tsx', ': unknown', ': any');
replaceInFile('hydragrow-frontend/src/pages/Settings.tsx', ': unknown', ': any');
replaceInFile('hydragrow-frontend/src/pages/DosingHistory.tsx', ': unknown', ': any');

// http.ts
replaceInFile('hydragrow-frontend/src/platform/http.ts', ': unknown', ': any');

// store/useDeviceStore.test.ts
replaceInFile('hydragrow-frontend/src/store/useDeviceStore.test.ts', ': unknown', ': any');

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
compFiles.forEach(c => replaceInFile(c, ': unknown', ': any'));
