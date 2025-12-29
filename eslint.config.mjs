// eslint.config.mjs
// ESLint v9 flat config, ESM, TS-aware, Sonar-style clean code
// Prettier handles formatting; ESLint handles correctness/clean-code.

import comments from '@eslint-community/eslint-plugin-eslint-comments/configs';
import js from '@eslint/js';
import nextPlugin from '@next/eslint-plugin-next';
import stylistic from '@stylistic/eslint-plugin';
import importPlugin from 'eslint-plugin-import';
import jsdoc from 'eslint-plugin-jsdoc';
import promise from 'eslint-plugin-promise';
import regexp from 'eslint-plugin-regexp';
import security from 'eslint-plugin-security';
import sonarjs from 'eslint-plugin-sonarjs';
import unicorn from 'eslint-plugin-unicorn';

import jest from 'eslint-plugin-jest';
import jestDom from 'eslint-plugin-jest-dom';
import jsxA11y from 'eslint-plugin-jsx-a11y';
import react from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';
import testingLibrary from 'eslint-plugin-testing-library';

import eslintConfigPrettier from 'eslint-config-prettier/flat';

import { defineConfig, globalIgnores } from 'eslint/config';
import tseslint from 'typescript-eslint';

// Constrain TS typed configs to TS/TSX only
const typedTsConfigs = [
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  ...tseslint.configs.recommendedTypeChecked,
].map((cfg) => ({
  ...cfg,
  files: ['**/*.{ts,tsx}'],
}));

export default defineConfig([
  // Global ignores
  globalIgnores([
    'node_modules/**',
    'dist/**',
    'build/**',
    'coverage/**',
    'app/AideonDesktop/src/design-system/components/**',
    '**/.pnpm/**',
    '**/out/**',
  ]),

  // Core JS recommendations (base for "clean code" checks)
  js.configs.recommended,

  // Next.js (core-web-vitals) recommended flat config
  {
    plugins: {
      '@next/next': nextPlugin,
    },
    rules: {
      ...nextPlugin.configs['core-web-vitals'].rules,
    },
  },

  // Base language options
  {
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
    },
    settings: {
      // Allow import/no-unresolved to pick up TS path aliases in package tsconfigs.
      'import/resolver': {
        typescript: {
          project: ['./app/AideonDesktop/tsconfig.json'],
          alwaysTryTypes: true,
        },
        node: true,
      },
    },
  },

  // Import hygiene (dead imports, cycles, etc.)
  importPlugin.flatConfigs.recommended,
  importPlugin.flatConfigs.typescript,

  // Promises, regex, unicorn (modern JS/TS patterns)
  promise.configs['flat/recommended'],
  regexp.configs['flat/recommended'],
  unicorn.configs.recommended,

  // Sonar-style code quality and complexity
  sonarjs.configs.recommended,

  // Security hotspot checks (not taint analysis, but still valuable)
  security.configs.recommended,

  // ESLint comments hygiene
  comments.recommended,

  // JSDoc quality (recommended ruleset)
  jsdoc.configs['flat/recommended'],

  // TypeScript typed configs (type-aware rules)
  ...typedTsConfigs,

  // React + JSX (component-level clean code)
  react.configs.flat.recommended,
  react.configs.flat['jsx-runtime'],

  // Accessibility for JSX
  jsxA11y.flatConfigs.recommended,

  // React Hooks rules
  {
    plugins: {
      'react-hooks': reactHooks,
    },
    rules: {
      // Use the plugin's recommended rules
      ...reactHooks.configs.recommended.rules,
      // Tighten the most important one
      'react-hooks/exhaustive-deps': 'error',
    },
  },

  // TS parser + TS-specific rules (on top of the typed presets)
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: {
        projectService: false,
        project: ['./app/AideonDesktop/tsconfig.json'],
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
      '@typescript-eslint/consistent-type-imports': 'error',
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/explicit-module-boundary-types': 'off',
    },
  },

  // Project-level Sonar-style tightening
  {
    rules: {
      // Keep cognitive complexity under control
      'sonarjs/cognitive-complexity': ['error', 8],
      // Optional: avoid huge switches
      'sonarjs/max-switch-cases': ['error', 10],
    },
  },

  // Avoid ESLint trying to sort imports; Prettier (with prettier-plugin-organize-imports)
  // is the single source of truth for import ordering.
  {
    name: 'no-eslint-import-sorting',
    rules: {
      'sort-imports': 'off',
      'import/order': 'off',
    },
  },
  {
    name: 'jsdoc-bypass',
    rules: {
      'jsdoc/require-param-type': 'off',
      'jsdoc/require-returns-type': 'off',
      'jsdoc/require-param-description': 'off',
      'jsdoc/require-returns': 'off',
      'jsdoc/check-param-names': 'off',
    },
  },

  // Test files: Jest + Testing Library + jest-dom
  {
    files: ['**/*.{test,spec}.{js,jsx,ts,tsx}'],

    // Jest recommended + style rules
    ...jest.configs['flat/recommended'],
    ...jest.configs['flat/style'],

    // React Testing Library best practices
    ...testingLibrary.configs['flat/react'],

    // jest-dom assertions best practices
    ...jestDom.configs['flat/recommended'],
  },

  // Put eslint-config-prettier LAST so it can disable any formatting rules
  // from the above configs that would conflict with Prettier.
  {
    plugins: {
      '@stylistic': stylistic,
    },
    rules: {
      // Core Sonar parity (requested list) -------------------------------------------------
      'max-len': ['error', { code: 120 }], // S103
      'no-console': 'error', // S106
      'no-empty': ['error', { allowEmptyCatch: false }], // S108
      'eol-last': ['error', 'always'], // S113
      curly: ['error', 'all'], // S121
      'max-statements-per-line': ['error', { max: 1 }], // S122
      'line-comment-position': ['error', { position: 'above' }], // S139
      'no-sequences': 'error', // S878
      'no-continue': 'error', // S909
      'jsx-a11y/iframe-has-title': 'error', // S1090
      '@typescript-eslint/no-shadow': 'error', // S1117
      'no-trailing-spaces': 'error', // S1131
      'no-unsafe-finally': 'error', // S1143
      'no-lone-blocks': 'error', // S1199
      'no-octal': 'error', // S1314
      'no-with': 'error', // S1321
      eqeqeq: ['error', 'always'], // S1440
      quotes: ['error', 'single', { avoidEscape: true }], // S1441
      'no-alert': 'error', // S1442
      'no-multi-str': 'error', // S1516
      'no-debugger': 'error', // S1525
      'no-dupe-args': 'error', // S1536
      '@stylistic/comma-dangle': ['error', 'always-multiline'], // S1537
      strict: ['error', 'never'], // S1539
      'no-self-assign': ['error', { props: true }], // S1656
      'no-unreachable-loop': 'error', // S1751
      'no-ternary': 'off', // S1774
      '@typescript-eslint/no-extraneous-class': 'error', // S2094
      radix: ['error', 'always'], // S2427
      'no-setter-return': 'error', // S2432
      'no-caller': 'error', // S2685
      '@typescript-eslint/prefer-readonly': 'error', // S2933
      '@typescript-eslint/no-non-null-assertion': 'error', // S2966
      '@typescript-eslint/no-inferrable-types': 'error', // S3257
      'prefer-const': 'error', // S3353
      'no-new-func': 'error', // S3523
      'no-template-curly-in-string': 'error', // S3786
      'no-empty-pattern': 'error', // S3799
      'no-unsafe-negation': 'error', // S3812
      'no-new-native-nonconstructor': 'error', // S3834
      'import/no-duplicates': 'error', // S3863
      '@typescript-eslint/await-thenable': 'error', // S4123
      '@typescript-eslint/no-misused-new': 'error', // S4124
      'valid-typeof': 'error', // S4125
      '@typescript-eslint/adjacent-overload-signatures': 'error', // S4136
      '@typescript-eslint/consistent-type-assertions': 'error', // S4137
      'no-sparse-arrays': 'error', // S4140
      '@typescript-eslint/no-unnecessary-type-arguments': 'error', // S4157
      '@typescript-eslint/no-explicit-any': 'warn', // S4204
      '@typescript-eslint/no-unnecessary-type-assertion': 'error', // S4325
      'no-return-await': 'error', // S4326
      'prefer-regex-literals': 'error', // S6325
      'react/require-render-return': 'error', // S6435
      'react/jsx-no-comment-textnodes': 'error', // S6438
      'react/jsx-no-bind': ['error', { allowArrowFunctions: true }], // S6480
      'no-extra-boolean-cast': 'error', // S6509
      'no-import-assign': 'error', // S6522
      'no-unsafe-optional-chaining': 'error', // S6523
      'no-loss-of-precision': 'error', // S6534
      '@typescript-eslint/prefer-literal-enum-member': 'error', // S6550
      '@typescript-eslint/prefer-return-this-type': 'error', // S6565
      '@typescript-eslint/no-confusing-non-null-assertion': 'error', // S6568
      '@typescript-eslint/no-unnecessary-type-constraint': 'error', // S6569
      '@typescript-eslint/no-duplicate-enum-values': 'error', // S6578
      '@typescript-eslint/no-mixed-enums': 'error', // S6583
      '@typescript-eslint/prefer-as-const': 'error', // S6590
      'no-constructor-return': 'error', // S6635
      'no-extra-bind': 'error', // S6637
      'no-constant-binary-expression': 'error', // S6638
      'no-unneeded-ternary': 'error', // S6644
      'no-undef-init': 'error', // S6645
      'no-useless-rename': 'error', // S6650
      'prefer-object-has-own': 'error', // S6653
      'no-proto': 'error', // S6654
      'no-octal-escape': 'error', // S6657
      '@typescript-eslint/prefer-promise-reject-errors': 'error', // S6671
      'react/no-direct-mutation-state': 'error', // S6746
      'react/no-children-prop': 'error', // S6748
      'react/no-render-return-value': 'error', // S6750
      'react/no-access-state-in-setstate': 'error', // S6756
      'react/no-this-in-sfc': 'error', // S6757
      'react/no-danger-with-children': 'error', // S6761
      'react/no-redundant-should-component-update': 'error', // S6763
      'react/no-unescaped-entities': 'error', // S6766
      'react/no-unused-prop-types': 'error', // S6767
      'react/jsx-pascal-case': 'error', // S6770
      'react/jsx-child-element-spacing': 'error', // S6772
      'react/prop-types': 'error', // S6774
      'react/default-props-match-prop-types': 'error', // S6775
      'react/no-is-mounted': 'error', // S6789
      'react/no-string-refs': 'error', // S6790
      'jsx-a11y/aria-proptypes': 'error', // S6793
      'jsx-a11y/role-has-required-aria-props': 'error', // S6807
      'jsx-a11y/role-supports-aria-props': 'error', // S6811
      'jsx-a11y/prefer-tag-over-role': 'error', // S6819
      'jsx-a11y/aria-role': 'error', // S6821
      'jsx-a11y/no-redundant-roles': 'error', // S6822
      'jsx-a11y/aria-activedescendant-has-tabindex': 'error', // S6823
      'jsx-a11y/aria-unsupported-elements': 'error', // S6824
      'jsx-a11y/no-aria-hidden-on-focusable': 'error', // S6825
      'no-case-declarations': 'error', // S6836
      'jsx-a11y/autocomplete-valid': 'error', // S6840
      'jsx-a11y/tabindex-no-positive': 'error', // S6841
      'jsx-a11y/no-noninteractive-element-to-interactive-role': 'error', // S6842
      'jsx-a11y/no-interactive-element-to-noninteractive-role': 'error', // S6843
      'jsx-a11y/no-noninteractive-tabindex': 'error', // S6845
      'jsx-a11y/no-access-key': 'error', // S6846
      'jsx-a11y/no-noninteractive-element-interactions': 'error', // S6847
      'jsx-a11y/no-static-element-interactions': 'error', // S6848
      'jsx-a11y/heading-has-content': 'error', // S6850
      'jsx-a11y/img-redundant-alt': 'error', // S6851
      'jsx-a11y/interactive-supports-focus': 'error', // S6852
      'import/no-absolute-path': 'error', // S6859
      'import/no-mutable-exports': 'error', // S6861
      // Angular rules (plugin not installed yet)
      // '@angular-eslint/contextual-lifecycle': 'error', // S7641
      // '@angular-eslint/no-empty-lifecycle-method': 'error', // S7647
      // '@angular-eslint/prefer-standalone': 'error', // S7648
      // '@angular-eslint/no-input-rename': 'error', // S7649
      // '@angular-eslint/no-inputs-metadata-property': 'error', // S7650
      // '@angular-eslint/no-output-native': 'error', // S7651
      // '@angular-eslint/no-output-on-prefix': 'error', // S7652
      // '@angular-eslint/no-output-rename': 'error', // S7653
      // '@angular-eslint/no-outputs-metadata-property': 'error', // S7654
      // '@angular-eslint/use-lifecycle-interface': 'error', // S7655
      // '@angular-eslint/use-pipe-transform-interface': 'error', // S7656
      'unicorn/catch-error-name': 'error', // S7718
      'unicorn/consistent-date-clone': 'error', // S7719
      'unicorn/consistent-empty-array-spread': 'error', // S7720
      'unicorn/consistent-function-scoping': 'error', // S7721
      'unicorn/error-message': 'error', // S7722
      'unicorn/new-for-builtins': 'error', // S7723
      'unicorn/no-abusive-eslint-disable': 'error', // S7724
      'unicorn/no-accessor-recursion': 'error', // S7725
      'unicorn/no-anonymous-default-export': 'error', // S7726
      'unicorn/no-array-callback-reference': 'error', // S7727
      'unicorn/no-array-for-each': 'error', // S7728
      'unicorn/no-array-method-this-argument': 'error', // S7729
      'unicorn/no-await-expression-member': 'error', // S7730
      'unicorn/no-for-loop': 'error', // S7731
      'unicorn/no-instanceof-builtins': 'error', // S7732
      'unicorn/no-invalid-fetch-options': 'error', // S7733
      'unicorn/no-named-default': 'error', // S7734
      'unicorn/no-negated-condition': 'error', // S7735
      'unicorn/no-negation-in-equality-check': 'error', // S7736
      'unicorn/no-object-as-default-parameter': 'error', // S7737
      'unicorn/no-single-promise-in-promise-methods': 'error', // S7738
      'unicorn/no-thenable': 'error', // S7739
      'unicorn/no-this-assignment': 'error', // S7740
      'unicorn/no-typeof-undefined': 'error', // S7741
      'unicorn/no-unnecessary-polyfills': 'error', // S7742
      'unicorn/no-unreadable-iife': 'error', // S7743
      'unicorn/no-useless-fallback-in-spread': 'error', // S7744
      'unicorn/no-useless-length-check': 'error', // S7745
      'unicorn/no-useless-promise-resolve-reject': 'error', // S7746
      'unicorn/no-useless-spread': 'error', // S7747
      'unicorn/no-zero-fractions': 'error', // S7748
      'unicorn/numeric-separators-style': 'error', // S7749
      'unicorn/prefer-array-find': 'error', // S7750
      'unicorn/prefer-array-flat': 'error', // S7751
      'unicorn/prefer-array-flat-map': 'error', // S7752
      'unicorn/prefer-array-index-of': 'error', // S7753
      'unicorn/prefer-array-some': 'error', // S7754
      'unicorn/prefer-at': 'error', // S7755
      'unicorn/prefer-blob-reading-methods': 'error', // S7756
      'unicorn/prefer-class-fields': 'error', // S7757
      'unicorn/prefer-code-point': 'error', // S7758
      'unicorn/prefer-date-now': 'error', // S7759
      'unicorn/prefer-default-parameters': 'error', // S7760
      'unicorn/prefer-dom-node-dataset': 'error', // S7761
      'unicorn/prefer-dom-node-remove': 'error', // S7762
      'unicorn/prefer-export-from': 'error', // S7763
      'unicorn/prefer-global-this': 'error', // S7764
      'unicorn/prefer-includes': 'error', // S7765
      'unicorn/prefer-math-min-max': 'error', // S7766
      'unicorn/prefer-math-trunc': 'error', // S7767
      'unicorn/prefer-modern-dom-apis': 'error', // S7768
      'unicorn/prefer-modern-math-apis': 'error', // S7769
      'unicorn/prefer-native-coercion-functions': 'error', // S7770
      'unicorn/prefer-negative-index': 'error', // S7771
      'unicorn/prefer-node-protocol': 'error', // S7772
      'unicorn/prefer-number-properties': 'error', // S7773
      'unicorn/prefer-prototype-methods': 'error', // S7774
      'unicorn/prefer-regexp-test': 'error', // S7775
      'unicorn/prefer-set-has': 'error', // S7776
      'unicorn/prefer-set-size': 'error', // S7777
      'unicorn/prefer-single-call': 'error', // S7778
      'unicorn/prefer-string-raw': 'error', // S7780
      'unicorn/prefer-string-replace-all': 'error', // S7781
      'unicorn/prefer-string-trim-start-end': 'error', // S7783
      'unicorn/prefer-structured-clone': 'error', // S7784
      'unicorn/prefer-top-level-await': 'error', // S7785
      'unicorn/prefer-type-error': 'error', // S7786
      'unicorn/require-module-specifiers': 'error', // S7787

      // Secondary table (SonarJS implementation mappings) ---------------------------------
      'max-params': ['error', 7], // S107
      '@typescript-eslint/no-magic-numbers': 'off', // S109
      '@typescript-eslint/switch-exhaustiveness-check': 'error', // S131
      '@typescript-eslint/no-unused-expressions': 'error', // S905
      'no-unused-private-class-members': 'error', // S1068
      'jsx-a11y/alt-text': 'error', // S1077
      'jsx-a11y/mouse-events-have-key-events': 'error', // S1082
      'jsx-a11y/click-events-have-key-events': 'error', // S1082
      'brace-style': ['error', '1tbs', { allowSingleLine: true }], // S1105
      '@stylistic/no-extra-semi': 'error', // S1116
      '@typescript-eslint/no-empty-function': 'error', // S1186
      '@stylistic/semi': ['error', 'always'], // S1438
      'no-dupe-keys': 'error', // S1534
      '@typescript-eslint/no-dupe-class-members': 'error', // S1534
      'react/jsx-no-duplicate-props': 'error', // S1534
      'no-unreachable': 'error', // S1763
      '@typescript-eslint/default-param-last': 'error', // S1788
      'no-unmodified-loop-condition': 'error', // S2189
      'accessor-pairs': ['error', { setWithoutGet: true }], // S2376
      'new-cap': ['error', { newIsCap: true, capIsNew: false }], // S2430
      'use-isnan': 'error', // S2688
      '@typescript-eslint/no-redeclare': 'error', // S2814
      'object-shorthand': 'error', // S3498
      'no-var': 'error', // S3504
      'prefer-template': 'error', // S3512
      'no-throw-literal': 'error', // S3696
      '@typescript-eslint/no-empty-interface': 'error', // S4023
      'jsx-a11y/media-has-caption': 'error', // S4084
      '@typescript-eslint/prefer-for-of': 'error', // S4138
      '@typescript-eslint/prefer-namespace-keyword': 'error', // S4156
      'getter-return': 'error', // S4275
      '@typescript-eslint/no-this-alias': 'error', // S4327
      'jsx-a11y/lang': 'error', // S5254
      'jsx-a11y/html-has-lang': 'error', // S5254
      'react-hooks/rules-of-hooks': 'error', // S6440
      'react/no-unused-class-component-methods': 'error', // S6441
      'react/jsx-key': 'error', // S6477
      'react/no-unstable-nested-components': 'error', // S6478
      'react/no-array-index-key': 'error', // S6479
      'react/jsx-no-constructed-context-values': 'error', // S6481
      'no-useless-escape': 'error', // S6535
      'no-nonoctal-decimal-escape': 'error', // S6535
      '@typescript-eslint/no-misused-promises': 'error', // S6544
      'no-async-promise-executor': 'error', // S6544
      '@typescript-eslint/no-base-to-string': 'error', // S6551
      '@typescript-eslint/prefer-string-starts-ends-with': 'error', // S6557
      '@typescript-eslint/no-redundant-type-constituents': 'error', // S6571
      '@typescript-eslint/prefer-enum-initializers': 'error', // S6572
      '@typescript-eslint/prefer-optional-chain': 'error', // S6582
      '@typescript-eslint/prefer-function-type': 'error', // S6598
      '@typescript-eslint/prefer-nullish-coalescing': 'error', // S6606
      'no-extend-native': 'error', // S6643
      'no-useless-constructor': 'error', // S6647
      'no-lonely-if': 'error', // S6660
      'prefer-object-spread': 'error', // S6661
      'prefer-spread': 'error', // S6666
      'no-useless-call': 'error', // S6676
      'no-self-compare': 'error', // S6679
      'react/no-unknown-property': 'error', // S6747
      'jsx-a11y/aria-props': 'error', // S6747
      'react/jsx-no-useless-fragment': 'error', // S6749
      'react/hook-use-state': 'error', // S6754
      'react/no-find-dom-node': 'error', // S6788
      'react/no-unsafe': 'error', // S6791
      'jsx-a11y/anchor-has-content': 'error', // S6827
      'jsx-a11y/anchor-is-valid': 'error', // S6844
      'jsx-a11y/label-has-associated-control': 'error', // S6853
      'react/no-deprecated': 'error', // S6957
      'import/no-self-import': 'error', // S7060
    },
  },
  eslintConfigPrettier,
]);
