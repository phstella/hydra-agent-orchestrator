/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_ALLOW_MOCK_IPC?: string;
}

declare module '*.css' {
  const content: Record<string, string>;
  export default content;
}
