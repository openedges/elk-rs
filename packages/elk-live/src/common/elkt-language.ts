/**
 * ELKT language definition for Monaco Editor.
 * Ported from the original elk-live Monarch tokenizer.
 *
 * Copyright (c) 2017 TypeFox GmbH (http://www.typefox.io) and others.
 * SPDX-License-Identifier: EPL-2.0
 */
import * as monaco from 'monaco-editor';

export function registerElktLanguage() {
  monaco.languages.register({
    id: 'elkt',
    extensions: ['.elkt'],
  });

  monaco.languages.setLanguageConfiguration('elkt', {
    comments: {
      lineComment: '//',
      blockComment: ['/*', '*/'],
    },
    brackets: [
      ['{', '}'],
      ['[', ']'],
    ],
    autoClosingPairs: [
      { open: '{', close: '}' },
      { open: '[', close: ']' },
    ],
  });

  monaco.languages.setMonarchTokensProvider('elkt', {
    keywords: [
      'graph', 'node', 'label', 'port', 'edge', 'layout', 'position', 'size',
      'incoming', 'outgoing', 'start', 'end', 'bends', 'section', 'true', 'false',
    ],

    typeKeywords: [],
    operators: [],

    symbols: /[=><!~?:&|+\-*/^%]+/,
    escapes: /\\(?:[btnfru\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8})/,

    tokenizer: {
      root: [
        [/[a-z_$][\w$]*/, {
          cases: {
            '@typeKeywords': 'keyword',
            '@keywords': 'keyword',
            '@default': 'identifier',
          },
        }],

        { include: '@whitespace' },

        [/[{}()[\]]/, '@brackets'],
        [/[<>](?!@symbols)/, '@brackets'],
        [/@symbols/, {
          cases: {
            '@operators': 'operator',
            '@default': '',
          },
        }],
      ],

      whitespace: [
        [/[ \t\r\n]+/, 'white'],
        [/\/\*/, 'comment', '@comment'],
        [/\/\/.*$/, 'comment'],
      ],

      comment: [
        [/[^/*]+/, 'comment'],
        [/\/\*/, 'comment.invalid'],
        [/\*\//, 'comment', '@pop'],
        [/[/*]/, 'comment'],
      ],

      string: [
        [/[^\\"]+/, 'string'],
        [/@escapes/, 'string.escape'],
        [/\\./, 'string.escape.invalid'],
        [/"/, 'string', '@pop'],
      ],
    },
  } as monaco.languages.IMonarchLanguage);
}
