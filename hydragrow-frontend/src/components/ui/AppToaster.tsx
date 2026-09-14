import { Toaster } from 'react-hot-toast';

export const AppToaster = () => (
  <Toaster
    position="top-center"
    containerClassName="app-toast-container"
    toastOptions={{
      duration: 3500,
      style: {
        borderRadius: '12px',
        background: 'var(--color-primary-deep)',
        color: '#FFFFFF',
        fontSize: '13px',
        fontWeight: 600,
        padding: '10px 14px',
        boxShadow: '0 10px 24px -8px rgba(20, 83, 45, 0.35)',
      },
      success: { iconTheme: { primary: 'var(--color-success-bg)', secondary: 'var(--color-primary-deep)' } },
      error: { iconTheme: { primary: 'var(--color-danger-bg)', secondary: '#7F1D1D' } },
    }}
  />
);