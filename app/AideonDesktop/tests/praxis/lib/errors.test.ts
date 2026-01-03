import { describe, expect, it } from 'vitest';

import { toErrorMessage } from 'praxis/lib/errors';

describe('toErrorMessage', () => {
  it('formats strings and error messages', () => {
    expect(toErrorMessage('plain')).toBe('plain');
    expect(toErrorMessage(new Error('boom'))).toBe('boom');
    expect(toErrorMessage({ code: 'validation_failed', message: 'Bad input' })).toBe('Bad input');
  });

  it('stringifies objects and falls back on circular structures', () => {
    expect(toErrorMessage({ a: 1 })).toBe('{"a":1}');
    const circular: { self?: unknown } = {};
    circular.self = circular;
    expect(toErrorMessage(circular)).toBe('[object Object]');
  });
});
