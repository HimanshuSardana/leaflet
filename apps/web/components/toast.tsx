"use client";

import { createContext, useCallback, useContext, useMemo, useState } from "react";

import {
  AnimatedToastStack,
  type AnimatedToast,
  type ToastStatus,
} from "@/components/motion/animated-toast-stack";

type ToastApi = {
  toast: (status: ToastStatus, title: string, description?: string) => void;
};

const ToastContext = createContext<ToastApi | null>(null);

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [items, setItems] = useState<AnimatedToast[]>([]);

  const dismiss = useCallback((id: string) => {
    setItems((current) => current.filter((item) => item.id !== id));
  }, []);

  const toast = useCallback((status: ToastStatus, title: string, description?: string) => {
    setItems((current) => [
      ...current,
      {
        id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
        status,
        title,
        description,
        createdAt: Date.now(),
      },
    ]);
  }, []);

  const value = useMemo(() => ({ toast }), [toast]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <AnimatedToastStack toasts={items} onDismiss={dismiss} position="bottom-right" />
    </ToastContext.Provider>
  );
}

export function useToast(): ToastApi {
  const context = useContext(ToastContext);
  if (!context) throw new Error("useToast must be used inside <ToastProvider>");
  return context;
}
