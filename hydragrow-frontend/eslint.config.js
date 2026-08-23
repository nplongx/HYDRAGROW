import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  {
    ignores: ["gleam_core/build/**"],
    rules: {
      "@typescript-eslint/no-unused-vars": "off",
      "no-empty": "off",
      "no-undef": "off",
      "@typescript-eslint/no-explicit-any": "off",
      "no-useless-escape": "off",
      "no-useless-assignment": "off",
      "@typescript-eslint/no-this-alias": "off",
      "@typescript-eslint/no-explicit-any": "off",
      '@typescript-eslint/no-explicit-any': 'warn',
    },
  }
);
