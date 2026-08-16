/// <reference types="vite/client" />
declare module '*gleam_core/*.mjs' {
  const content: any;
  export = content;
}

declare module '*gleam_core/settings/*.mjs' {
  const content: any;
  export = content;
}
