import { type ReactNode } from "react";
import { LoaderCircle } from "lucide-react";

import { cn } from "design-system/lib/utilities";

export type NodeStatus = "loading" | "success" | "error" | "initial";

export type NodeStatusVariant = "overlay" | "border";

export interface NodeStatusIndicatorProps {
  status?: NodeStatus;
  variant?: NodeStatusVariant;
  children: ReactNode;
}

export const SpinnerLoadingIndicator = ({
  children,
}: {
  children: ReactNode;
}) => {
  return (
    <div className="relative">
      <StatusBorder className="border-primary/40">{children}</StatusBorder>

      <div className="bg-background/50 absolute inset-0 z-50 rounded-[9px] backdrop-blur-xs" />
      <div className="absolute inset-0 z-50">
        <span className="absolute top-[calc(50%-1.25rem)] left-[calc(50%-1.25rem)] inline-block h-10 w-10 animate-ping rounded-full bg-primary/20" />

        <LoaderCircle className="absolute top-[calc(50%-0.75rem)] left-[calc(50%-0.75rem)] size-6 animate-spin text-primary" />
      </div>
    </div>
  );
};

export const BorderLoadingIndicator = ({
  children,
}: {
  children: ReactNode;
}) => {
  return (
    <>
      <div className="absolute -top-px -left-px h-[calc(100%+2px)] w-[calc(100%+2px)]">
        <style>
          {`
        @keyframes spin {
          from { transform: translate(-50%, -50%) rotate(0deg); }
          to { transform: translate(-50%, -50%) rotate(360deg); }
        }
        .spinner {
          animation: spin 2s linear infinite;
          position: absolute;
          left: 50%;
          top: 50%;
          width: 140%;
          aspect-ratio: 1;
          transform-origin: center;
        }
      `}
        </style>
        <div className="absolute inset-0 overflow-hidden rounded-[9px]">
          <div
            className="spinner rounded-full"
            style={{
              background:
                "conic-gradient(from 0deg at 50% 50%, var(--primary) 0deg, transparent 360deg)",
            }}
          />
        </div>
      </div>
      {children}
    </>
  );
};

const StatusBorder = ({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) => {
  return (
    <>
      <div
        className={cn(
          "absolute -top-px -left-px h-[calc(100%+2px)] w-[calc(100%+2px)] rounded-[9px] border-2",
          className,
        )}
      />
      {children}
    </>
  );
};

export const NodeStatusIndicator = ({
  status,
  variant = "border",
  children,
}: NodeStatusIndicatorProps) => {
  switch (status) {
    case "loading": {
      switch (variant) {
        case "overlay": {
          return <SpinnerLoadingIndicator>{children}</SpinnerLoadingIndicator>;
        }
        case "border": {
          return <BorderLoadingIndicator>{children}</BorderLoadingIndicator>;
        }
        default: {
          return <>{children}</>;
        }
      }
    }
    case "success": {
      return (
        <StatusBorder className="border-primary/60">{children}</StatusBorder>
      );
    }
    case "error": {
      return <StatusBorder className="border-destructive">{children}</StatusBorder>;
    }
    default: {
      return <>{children}</>;
    }
  }
};
