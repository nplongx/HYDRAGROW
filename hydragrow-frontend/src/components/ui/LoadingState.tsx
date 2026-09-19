import React from "react";

interface LoadingStateProps {
  message?: string;
  card?: boolean;
}

export const LoadingState: React.FC<LoadingStateProps> = ({
  message = "Đang tải...",
  card = false,
}) => {
  if (card) {
    return (
      <div
        className="ui-loading-card"
        role="status"
        aria-live="polite"
        aria-busy="true"
      >
        <div className="ui-loading-spinner" />
        <p className="ui-loading-message">{message}</p>
      </div>
    );
  }
  return (
    <div
      className="ui-loading-fullscreen"
      role="status"
      aria-live="polite"
      aria-busy="true"
    >
      <div className="ui-loading-spinner" />
      <p className="ui-loading-message">{message}</p>
    </div>
  );
};
