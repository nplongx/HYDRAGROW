import { Toaster } from 'react-hot-toast';

export const AppToaster = () => (
  <Toaster
    position="top-center"
    containerClassName="app-toast-container"
    toastOptions={{
      duration: 3500,
      style: {
        borderRadius: '12px',
        background: '#14532D',
        color: '#FFFFFF',
        fontSize: '13px',
        fontWeight: 600,
        padding: '10px 14px',
        boxShadow: '0 10px 24px -8px rgba(20, 83, 45, 0.35)',
      },
      success: { iconTheme: { primary: '#D1FAE5', secondary: '#14532D' } },
      error: { iconTheme: { primary: '#FEE2E2', secondary: '#7F1D1D' } },
    }}
  />
);