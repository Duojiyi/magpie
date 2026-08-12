import { Component, type ErrorInfo, type ReactNode } from "react";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  message: string;
}

/**
 * App-wide error boundary (audit P1: previously any render exception — e.g. a bad
 * persisted `language` reaching `t()` — took down the whole window with no recovery).
 *
 * Intentionally dependency-free: it must render even when the thing that threw is the
 * translation/theme layer itself, so it uses static bilingual text and a hard reload
 * rather than `t()` or app state.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, message: "" };
  }

  static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
    const message =
      error instanceof Error ? error.message : typeof error === "string" ? error : "Unknown error";
    return { hasError: true, message };
  }

  componentDidCatch(error: unknown, info: ErrorInfo): void {
    console.error("[ErrorBoundary] Uncaught render error:", error, info?.componentStack);
  }

  private handleReload = (): void => {
    window.location.reload();
  };

  render(): ReactNode {
    if (!this.state.hasError) {
      return this.props.children;
    }

    return (
      <div
        role="alert"
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 12,
          height: "100vh",
          padding: 24,
          textAlign: "center",
          fontFamily: "system-ui, sans-serif",
          color: "#e5e7eb",
          background: "#1f2430",
        }}
      >
        <div style={{ fontSize: 15, fontWeight: 600 }}>
          出了点问题 / Something went wrong
        </div>
        <div style={{ fontSize: 12, opacity: 0.7, maxWidth: 320, wordBreak: "break-word" }}>
          {this.state.message}
        </div>
        <button
          type="button"
          onClick={this.handleReload}
          style={{
            marginTop: 8,
            padding: "6px 16px",
            fontSize: 13,
            borderRadius: 6,
            border: "1px solid #4b5563",
            background: "#374151",
            color: "#f9fafb",
            cursor: "pointer",
          }}
        >
          重新加载 / Reload
        </button>
      </div>
    );
  }
}

export default ErrorBoundary;
