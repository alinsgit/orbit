import js from "@eslint/js";
import tsPlugin from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import reactPlugin from "eslint-plugin-react";
import reactHooksPlugin from "eslint-plugin-react-hooks";

export default [
  js.configs.recommended,
  {
    ignores: [
      "node_modules/**",
      "dist/**",
      "core/target/**",
      ".tauri/**",
      "docs/**",
      "tests/**",
    ],
  },
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: "latest",
        sourceType: "module",
        ecmaFeatures: { jsx: true },
      },
      globals: {
        console: "readonly",
        setTimeout: "readonly",
        clearTimeout: "readonly",
        setInterval: "readonly",
        clearInterval: "readonly",
        document: "readonly",
        window: "readonly",
        navigator: "readonly",
        fetch: "readonly",
        URL: "readonly",
        React: "readonly",
        HTMLElement: "readonly",
        HTMLDivElement: "readonly",
        HTMLInputElement: "readonly",
        HTMLTextAreaElement: "readonly",
        HTMLSelectElement: "readonly",
        Event: "readonly",
        KeyboardEvent: "readonly",
        MouseEvent: "readonly",
        MutationObserver: "readonly",
        ResizeObserver: "readonly",
        IntersectionObserver: "readonly",
        RequestAnimationFrame: "readonly",
        requestAnimationFrame: "readonly",
        cancelAnimationFrame: "readonly",
        Blob: "readonly",
        FileReader: "readonly",
        AbortController: "readonly",
        TextDecoder: "readonly",
        TextEncoder: "readonly",
        ReadableStreamDefaultReader: "readonly",
        crypto: "readonly",
        localStorage: "readonly",
        performance: "readonly",
      },
    },
    plugins: {
      "@typescript-eslint": tsPlugin,
      react: reactPlugin,
      "react-hooks": reactHooksPlugin,
    },
    rules: {
      // TypeScript
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/no-explicit-any": "off",

      // React
      "react/react-in-jsx-scope": "off",
      "react/prop-types": "off",
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",

      // General
      "preserve-caught-error": "off",
      "no-console": "off",
      "prefer-const": "warn",
      "no-var": "error",
    },
    settings: {
      react: { version: "detect" },
    },
  },
];
