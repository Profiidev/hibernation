import { describe, expect, it } from 'vitest';
import { linesToPatterns, patternsToLines } from '$routes/tokens/rules';

describe('token rule patterns', () => {
  it('splits lines and drops blanks', () => {
    expect(linesToPatterns(' ^myapp- \n\n-dev$\n')).toEqual([
      '^myapp-',
      '-dev$'
    ]);
    expect(linesToPatterns('')).toEqual([]);
  });

  it('round trips', () => {
    expect(linesToPatterns(patternsToLines(['^a', 'b$']))).toEqual([
      '^a',
      'b$'
    ]);
  });
});
